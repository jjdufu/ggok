use super::{Agent, Inner};
use anyhow::{Context, Result, bail};
use ggok_core::occupy::{
    self, LeaderRecord, leftover_idle, leftover_noleader_pid, read_leader_record, terminate_pid,
    write_leader_record,
};
use ggok_core::scan;
use serde_json::{Value, json};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn};

impl Agent {
    pub async fn shutdown(&self) {
        let mut g = self.inner.lock().await;
        release_client(&mut g);
    }

    /// Connect an ACP client if a leader is already reachable. Does not spawn a leader.
    ///
    /// # Errors
    /// Returns an error if the existing leader cannot be reached or the ACP client fails to start.
    pub async fn connect_existing_leader(&self) -> Result<()> {
        self.ensure_inner(false).await?;
        self.attach_running_sessions().await;
        Ok(())
    }

    pub(crate) async fn ensure(&self) -> Result<()> {
        self.ensure_inner(true).await
    }

    async fn ensure_inner(&self, spawn_leader: bool) -> Result<()> {
        let spawned = {
            let mut g = self.inner.lock().await;
            if g.initialized && child_alive(g.child_pid) {
                return Ok(());
            }
            if child_alive(g.child_pid) {
                false
            } else {
                self.reap_leftover_locked().await;
                self.connect_leader_and_spawn_acp(&mut g, spawn_leader)
                    .await?
            }
        };
        if spawned {
            self.initialize_client().await?;
            let acp = self.inner.lock().await.child_pid;
            self.persist_leader_record(false, acp);
            return Ok(());
        }
        {
            let g = self.inner.lock().await;
            if g.initialized && child_alive(g.child_pid) {
                return Ok(());
            }
            if child_alive(g.child_pid) {
                drop(g);
                return self.wait_ready().await;
            }
        }
        Ok(())
    }

    async fn reap_leftover_locked(&self) {
        let Some(pid) = leftover_noleader_pid(&self.pid_file) else {
            return;
        };
        if !leftover_idle(&self.grok_home) {
            return;
        }
        terminate_pid(pid, "TERM");
        tokio::time::sleep(Duration::from_millis(200)).await;
        if occupy::pid_alive(pid) {
            terminate_pid(pid, "KILL");
        }
        let _ = std::fs::remove_file(&self.pid_file);
    }

    async fn connect_leader_and_spawn_acp(
        &self,
        inner: &mut Inner,
        spawn_leader: bool,
    ) -> Result<bool> {
        let sock = leader_socket(&self.grok_home);
        if !self.leader_supported() {
            if spawn_leader {
                bail!("grok binary has no leader subcommand");
            }
            return Ok(false);
        }
        let spawned_leader = match reachable_leader(&self.grok_bin, &self.grok_home, &sock) {
            Some(_) => false,
            None if spawn_leader => {
                self.spawn_leader(&sock).await?;
                true
            }
            None => return Ok(false),
        };
        self.wait_leader(&sock).await?;
        if let Some(pid) = reachable_leader(&self.grok_bin, &self.grok_home, &sock) {
            info!(pid, spawned_leader, "grok leader reachable");
        }
        self.persist_leader_record(spawned_leader, None);
        self.spawn_acp(inner, &sock)?;
        self.persist_leader_record(spawned_leader, inner.child_pid);
        Ok(true)
    }

    async fn spawn_leader(&self, sock: &Path) -> Result<()> {
        if setsid_can_fork() {
            self.spawn_leader_setsid(sock).await
        } else {
            self.spawn_leader_shell(sock).await
        }
    }

    async fn spawn_leader_setsid(&self, sock: &Path) -> Result<()> {
        let log = open_append(&self.leader_stderr_log())?;
        let mut cmd = Command::new("setsid");
        cmd.arg("-f");
        apply_leader_args(&mut cmd, &self.grok_bin, sock, &self.grok_home);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .kill_on_drop(false);
        let status = cmd.status().await.context("spawn grok agent leader")?;
        if !status.success() {
            bail!("setsid grok agent leader exited {status}");
        }
        Ok(())
    }

    async fn spawn_leader_shell(&self, sock: &Path) -> Result<()> {
        let log_path = self.leader_stderr_log();
        let _ = open_append(&log_path)?;
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(
                r#""$GGOK_GROK_BIN" agent leader --no-exit-on-disconnect --no-auto-update --leader-socket "$GGOK_LEADER_SOCK" </dev/null >>"$GGOK_LOG" 2>&1 &"#,
            )
            .env("GGOK_GROK_BIN", &self.grok_bin)
            .env("GGOK_LEADER_SOCK", sock)
            .env("GGOK_LOG", &log_path)
            .env("GROK_HOME", &self.grok_home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(false);
        let status = cmd.status().await.context("spawn grok agent leader")?;
        if !status.success() {
            bail!("shell grok agent leader exited {status}");
        }
        Ok(())
    }

    fn persist_leader_record(&self, spawned_now: bool, acp_pid: Option<u32>) {
        let sock = leader_socket(&self.grok_home);
        let Some(pid) = reachable_leader(&self.grok_bin, &self.grok_home, &sock) else {
            return;
        };
        let held = acp_pid.is_some_and(occupy::stdio_holds_leader)
            || acp_pid.is_some_and(|p| ggok_core::sys::pid_ppid(pid) == Some(p));
        let independent = occupy::leader_is_independent(pid) && !held;
        let prev = read_leader_record(&self.leader_json);
        let owned = independent && (spawned_now || prev.is_some_and(|p| p.owned && p.pid == pid));
        if held {
            warn!(
                pid,
                acp_pid,
                "grok leader is a child of the ACP client; that client will not be killed"
            );
        }
        let _ = write_leader_record(
            &self.leader_json,
            &LeaderRecord {
                pid,
                owned,
                socket: sock.to_string_lossy().into_owned(),
            },
        );
    }

    fn leader_stderr_log(&self) -> PathBuf {
        self.leader_json
            .parent()
            .unwrap_or(Path::new("."))
            .join("ggok.log")
    }

    async fn wait_leader(&self, sock: &Path) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if reachable_leader(&self.grok_bin, &self.grok_home, sock).is_some() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!("timeout waiting for grok leader");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn spawn_acp(&self, inner: &mut Inner, sock: &Path) -> Result<()> {
        release_client(inner);
        let mut cmd = Command::new(&self.grok_bin);
        cmd.arg("agent")
            .arg("--leader")
            .arg("--leader-socket")
            .arg(sock)
            .arg("stdio")
            .env("GROK_HOME", &self.grok_home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false);
        let mut child = cmd.spawn().context("spawn grok agent stdio client")?;
        let stdin = child.stdin.take().context("grok stdin")?;
        let stdout = child.stdout.take().context("grok stdout")?;
        let stderr = child.stderr.take();
        let pid = child.id();
        inner.child = Some(child);
        inner.stdin = Some(stdin);
        inner.initialized = false;
        inner.pending.clear();
        inner.in_flight.clear();
        inner.child_pid = pid;
        for sess in inner.sessions.values_mut() {
            sess.loaded = false;
            sess.running = false;
        }
        if let Some(err) = stderr {
            tokio::spawn(async move {
                let mut lines = BufReader::new(err).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() {
                        tracing::warn!("grok stderr: {line}");
                    }
                }
            });
        }
        let agent = self.clone();
        tokio::spawn(async move {
            agent.read_stdout(pid, stdout).await;
        });
        info!(pid = pid.unwrap_or(0), "grok acp stdio client started");
        Ok(())
    }

    async fn initialize_client(&self) -> Result<()> {
        {
            let g = self.inner.lock().await;
            if g.initialized && child_alive(g.child_pid) {
                return Ok(());
            }
            if !child_alive(g.child_pid) {
                bail!("grok acp client exited before initialize");
            }
        }
        let init = self
            .call(
                "initialize",
                json!({
                    "protocolVersion": 1,
                    "clientInfo": { "name": "ggok", "version": env!("CARGO_PKG_VERSION") },
                    "clientCapabilities": {
                        "fs": { "readTextFile": false, "writeTextFile": false },
                        "terminal": false
                    }
                }),
            )
            .await?;
        self.apply_initialize(&init).await;
        let _ = self
            .call("authenticate", json!({ "methodId": "cached_token" }))
            .await;
        self.inner.lock().await.initialized = true;
        Ok(())
    }

    async fn wait_ready(&self) -> Result<()> {
        for _ in 0..200 {
            {
                let g = self.inner.lock().await;
                if g.initialized && child_alive(g.child_pid) {
                    return Ok(());
                }
                if !child_alive(g.child_pid) {
                    bail!("grok acp client exited during initialize");
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        bail!("timeout waiting for grok acp client")
    }

    pub(crate) async fn attach_running_sessions(&self) {
        {
            let g = self.inner.lock().await;
            if !g.initialized || !child_alive(g.child_pid) {
                return;
            }
        }
        if leftover_noleader_pid(&self.pid_file).is_some() {
            return;
        }
        let id = self.inner.lock().await.web_active_id.clone();
        let Some(id) = id else {
            return;
        };
        let Ok(index) = scan::scan(&self.grok_home) else {
            return;
        };
        let s3 = occupy::cli_sessions(&self.grok_home);
        let our = occupy::our_runtime_pid(self.inner.lock().await.child_pid);
        if occupy::tui_held(&s3, &id, our) {
            return;
        }
        let Some(meta) = index.get(&id) else {
            return;
        };
        if let Err(e) = self.session_load_inner(&id, &meta.cwd).await {
            warn!("attach {id}: {e:#}");
        }
    }

    #[must_use]
    pub async fn can_attach(&self) -> bool {
        if leftover_noleader_pid(&self.pid_file).is_some() {
            return false;
        }
        let g = self.inner.lock().await;
        g.initialized && child_alive(g.child_pid)
    }

    pub async fn occupancy_of(&self, id: &str, cwd: Option<&str>) -> occupy::Occupancy {
        let live = self.live_view(id).await;
        let our = occupy::our_runtime_pid(self.inner.lock().await.child_pid);
        let leftover = leftover_noleader_pid(&self.pid_file).is_some();
        let can_attach = {
            let g = self.inner.lock().await;
            g.initialized && child_alive(g.child_pid) && !leftover
        };
        let s3 = occupy::cli_sessions(&self.grok_home);
        let dir = cwd
            .filter(|c| !c.is_empty())
            .map(|c| session_dir(&self.grok_home, c, id))
            .or_else(|| {
                scan::scan(&self.grok_home)
                    .ok()
                    .and_then(|idx| idx.get(id).map(|m| m.dir.clone()))
            });
        let jsonl = dir.as_ref().is_some_and(|d| occupy::jsonl_running(d));
        occupy::classify(&occupy::ClassifyInput {
            id,
            live: live.as_ref(),
            our_runtime_pid: our,
            s3: &s3,
            leftover_noleader_alive: leftover,
            jsonl_running: jsonl,
            can_attach,
        })
    }

    fn leader_supported(&self) -> bool {
        std::process::Command::new(&self.grok_bin)
            .args(["agent", "leader", "--help"])
            .env("GROK_HOME", &self.grok_home)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    pub(crate) async fn read_stdout(&self, pid: Option<u32>, stdout: tokio::process::ChildStdout) {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                warn!("grok non-json: {}", super::session::truncate(&line, 200));
                continue;
            };
            if let Err(e) = self.on_message(msg).await {
                warn!("grok message: {e:#}");
            }
        }
        let mut g = self.inner.lock().await;
        if pid.is_some() && g.child_pid != pid {
            warn!(
                old_pid = pid.unwrap_or(0),
                current_pid = g.child_pid.unwrap_or(0),
                "stale grok agent stdout closed"
            );
            return;
        }
        g.initialized = false;
        for (_, tx) in g.pending.drain() {
            let _ = tx.send(Err("grok agent exited".into()));
        }
        g.in_flight.clear();
        for sess in g.sessions.values_mut() {
            sess.running = false;
            sess.loaded = false;
        }
        warn!(pid = pid.unwrap_or(0), "grok agent stdout closed");
    }
}

fn release_client(inner: &mut Inner) {
    let acp_pid = inner.child_pid;
    let held = acp_pid.is_some_and(occupy::stdio_holds_leader);
    inner.stdin = None;
    inner.initialized = false;
    inner.child_pid = None;
    let Some(mut child) = inner.child.take() else {
        return;
    };
    if !held {
        let _ = child.start_kill();
    }
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
}

pub(crate) fn child_alive(pid: Option<u32>) -> bool {
    pid.is_some_and(ggok_core::sys::pid_is_alive)
}

fn leader_socket(grok_home: &Path) -> PathBuf {
    grok_home.join("leader.sock")
}

fn session_dir(grok_home: &Path, cwd: &str, id: &str) -> PathBuf {
    grok_home
        .join("sessions")
        .join(
            percent_encoding::utf8_percent_encode(cwd, percent_encoding::NON_ALPHANUMERIC)
                .to_string(),
        )
        .join(id)
}

fn reachable_leader(bin: &Path, grok_home: &Path, sock: &Path) -> Option<u32> {
    let out = std::process::Command::new(bin)
        .args([
            "leader",
            "list",
            "--json",
            "--leader-socket",
            &sock.to_string_lossy(),
        ])
        .env("GROK_HOME", grok_home)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    occupy::first_reachable_leader_pid(&String::from_utf8_lossy(&out.stdout))
}

fn apply_leader_args(cmd: &mut Command, grok_bin: &Path, sock: &Path, grok_home: &Path) {
    cmd.arg(grok_bin)
        .arg("agent")
        .arg("leader")
        .arg("--no-exit-on-disconnect")
        .arg("--no-auto-update")
        .arg("--leader-socket")
        .arg(sock)
        .env("GROK_HOME", grok_home);
}

fn open_append(path: &Path) -> Result<File> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    File::options()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))
}

fn setsid_can_fork() -> bool {
    let probe = if Path::new("/bin/true").is_file() {
        "/bin/true"
    } else {
        "true"
    };
    std::process::Command::new("setsid")
        .args(["-f", probe])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
