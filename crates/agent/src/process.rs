use super::{Agent, Inner};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn};

impl Agent {
    pub async fn shutdown(&self) {
        let mut g = self.inner.lock().await;
        stop_child(&mut g, &self.pid_file);
    }

    pub(crate) async fn ensure(&self) -> Result<()> {
        {
            let g = self.inner.lock().await;
            if g.initialized && child_alive(g.child_pid) {
                return Ok(());
            }
        }
        self.start().await
    }

    pub(crate) async fn start(&self) -> Result<()> {
        kill_stale(&self.pid_file);
        {
            let mut g = self.inner.lock().await;
            stop_child(&mut g, &self.pid_file);
        }
        let mut cmd = if command_exists("setsid") {
            let mut c = Command::new("setsid");
            c.arg(&self.grok_bin);
            c
        } else {
            Command::new(&self.grok_bin)
        };
        cmd.arg("agent")
            .arg("--no-leader")
            .arg("stdio")
            .env("GROK_HOME", &self.grok_home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn().context("spawn grok agent stdio")?;
        let stdin = child.stdin.take().context("grok stdin")?;
        let stdout = child.stdout.take().context("grok stdout")?;
        let stderr = child.stderr.take();
        let pid = child.id();
        {
            let mut g = self.inner.lock().await;
            g.child = Some(child);
            g.stdin = Some(stdin);
            g.initialized = false;
            g.pending.clear();
            g.in_flight.clear();
            g.child_pid = pid;
            for sess in g.sessions.values_mut() {
                sess.loaded = false;
                sess.running = false;
            }
        }
        if let Some(pid) = pid {
            let _ = fs::write(&self.pid_file, format!("{pid}\n"));
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
        info!(pid = pid.unwrap_or(0), "grok agent stdio started");
        Ok(())
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

pub(crate) fn stop_child(inner: &mut Inner, pid_file: &Path) {
    if let Some(pid) = inner.child_pid {
        if command_exists("setsid") {
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(format!("-{pid}"))
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        let _ = std::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    if let Some(child) = inner.child.as_mut() {
        let _ = child.start_kill();
    }
    inner.child = None;
    inner.stdin = None;
    inner.initialized = false;
    inner.child_pid = None;
    let _ = fs::remove_file(pid_file);
}

pub(crate) fn kill_stale(pid_file: &Path) {
    let Ok(raw) = fs::read_to_string(pid_file) else {
        return;
    };
    let Ok(pid) = raw.trim().parse::<u32>() else {
        let _ = fs::remove_file(pid_file);
        return;
    };
    if !child_alive(Some(pid)) {
        let _ = fs::remove_file(pid_file);
        return;
    }
    let cmd = ggok_core::sys::pid_cmdline(pid);
    let text = String::from_utf8_lossy(&cmd);
    if text.contains("stdio") || text.contains("grok") {
        if command_exists("setsid") {
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(format!("-{pid}"))
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        let _ = std::process::Command::new("kill")
            .arg("-KILL")
            .arg(pid.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = fs::remove_file(pid_file);
}

pub(crate) fn child_alive(pid: Option<u32>) -> bool {
    pid.is_some_and(ggok_core::sys::pid_is_alive)
}

pub(crate) fn command_exists(name: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {name} >/dev/null")])
        .status()
        .is_ok_and(|s| s.success())
}
