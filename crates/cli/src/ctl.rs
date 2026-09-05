use crate::config::StartArgs;
use anyhow::{bail, Context, Result};
use daemonize::Daemonize;
use ggok_core::config::{
    self, display_token, log_file, pid_file, read_saved_state, running_pid, RuntimeConfig,
};
use ggok_core::paths::UPLOAD_DIR;
use ggok_core::{agent_pid_file, effective_uid, pid_is_alive};
use ggok_server::Service;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn start(args: &StartArgs) -> Result<()> {
    let cfg = RuntimeConfig::prepare(args.clone().into_overrides())?;
    if let Some(pid) = running_pid(&cfg.pid_file) {
        print_report(
            "already running",
            Some(pid),
            &cfg.bind,
            &cfg.log_file,
            &cfg.grok_home,
        );
        return Ok(());
    }
    if cfg.pid_file.exists() {
        fs::remove_file(&cfg.pid_file)
            .with_context(|| format!("remove stale pid file {}", cfg.pid_file.display()))?;
    }
    let child = spawn_worker(args)?;
    let pid = wait_for_start(&cfg, child)?;
    print_report(
        "started",
        Some(pid),
        &cfg.bind,
        &cfg.log_file,
        &cfg.grok_home,
    );
    Ok(())
}

pub fn restart(args: &StartArgs) -> Result<()> {
    stop()?;
    start(args)
}

pub fn stop() -> Result<()> {
    stop_with_output(true)
}

fn stop_with_output(print: bool) -> Result<()> {
    let pid_path = pid_file()?;
    kill_stale_agent();
    let Some(pid) = running_pid(&pid_path) else {
        let _ = fs::remove_file(&pid_path);
        if print {
            println!("not running");
        }
        return Ok(());
    };
    signal(pid, "TERM");
    for _ in 0..10 {
        if !pid_is_alive(pid) {
            let _ = fs::remove_file(&pid_path);
            if print {
                println!("stopped pid={pid}");
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    signal(pid, "KILL");
    let _ = fs::remove_file(&pid_path);
    if print {
        println!("killed pid={pid}");
    }
    Ok(())
}

/// Stop ggok, then delete its binary, config, logs, pid files, and upload cache.
/// Does not touch `~/.grok` or workspace files.
///
/// # Errors
/// Returns an error if `HOME` cannot be resolved while locating data dirs.
pub fn uninstall() -> Result<i32> {
    if let Err(err) = stop_with_output(false) {
        eprintln!("stop: {err:#}");
    }

    let mut leftover = Vec::new();
    if let Ok(dir) = config::config_dir() {
        remove_ggok_tree(&dir, &mut leftover);
    }
    if let Ok(dir) = config::state_dir() {
        remove_ggok_tree(&dir, &mut leftover);
    }
    remove_ggok_tree(Path::new(UPLOAD_DIR), &mut leftover);

    for bin in ggok_bin_candidates() {
        remove_ggok_bin(&bin, &mut leftover);
    }

    leftover.sort();
    leftover.dedup();
    leftover.retain(|p| p.exists() || p.symlink_metadata().is_ok());

    if leftover.is_empty() {
        println!("uninstalled");
        println!("Grok CLI data under ~/.grok was not removed");
        Ok(0)
    } else {
        eprintln!("uninstalled with leftovers:");
        for path in &leftover {
            eprintln!("  {}", path.display());
        }
        eprintln!("remove those paths manually (sudo if needed)");
        Ok(1)
    }
}

fn ggok_bin_candidates() -> Vec<PathBuf> {
    let mut out = BTreeSet::new();
    if let Some(home) = std::env::var_os("HOME") {
        out.insert(PathBuf::from(home).join(".local/bin/ggok"));
    }
    out.insert(PathBuf::from("/usr/local/bin/ggok"));
    if let Ok(exe) = std::env::current_exe()
        && exe.file_name().and_then(|n| n.to_str()) == Some("ggok")
        && !is_build_artifact(&exe)
    {
        out.insert(exe);
    }
    if let Some(path) = which_ggok()
        && !is_build_artifact(&path)
    {
        out.insert(path);
    }
    out.into_iter().collect()
}

fn which_ggok() -> Option<PathBuf> {
    let output = Command::new("sh")
        .args(["-c", "command -v ggok"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8(output.stdout).ok()?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn is_build_artifact(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "target")
}

fn remove_ggok_tree(path: &Path, leftover: &mut Vec<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if !is_ggok_leaf(path) {
        leftover.push(path.to_path_buf());
        return;
    }
    if effective_uid() != Some(meta.uid()) {
        leftover.push(path.to_path_buf());
        return;
    }
    let result = if meta.file_type().is_symlink() || meta.is_file() {
        fs::remove_file(path)
    } else if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        leftover.push(path.to_path_buf());
        return;
    };
    match result {
        Ok(()) => println!("removed {}", path.display()),
        Err(_) => leftover.push(path.to_path_buf()),
    }
}

fn remove_ggok_bin(path: &Path, leftover: &mut Vec<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if path.file_name().and_then(|n| n.to_str()) != Some("ggok") {
        return;
    }
    if is_build_artifact(path) {
        return;
    }
    if effective_uid() != Some(meta.uid()) {
        leftover.push(path.to_path_buf());
        return;
    }
    match fs::remove_file(path) {
        Ok(()) => println!("removed {}", path.display()),
        Err(_) => leftover.push(path.to_path_buf()),
    }
}

fn is_ggok_leaf(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("ggok" | ".ggok-uploads")
    )
}

pub fn status() -> Result<i32> {
    let pid_path = pid_file()?;
    let saved = read_saved_state();
    let log = saved.as_ref().map_or(log_file()?, |s| s.log_file.clone());
    let bind = saved
        .as_ref()
        .map_or_else(|| config::DEFAULT_BIND.to_string(), |s| s.bind.clone());
    let grok_home = saved
        .as_ref()
        .map_or(config::default_grok_home()?, |s| s.grok_home.clone());
    if let Some(pid) = running_pid(&pid_path) {
        print_report("running", Some(pid), &bind, &log, &grok_home);
        Ok(0)
    } else {
        print_report("not running", None, &bind, &log, &grok_home);
        Ok(3)
    }
}

pub fn daemon(args: StartArgs) -> Result<()> {
    let cfg = RuntimeConfig::from_overrides(args.into_overrides())?;
    enter_daemon(&cfg)?;
    cfg.write_saved_state()?;
    run_server(cfg)
}

pub(crate) fn run_server(cfg: RuntimeConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("tokio runtime")?;
    let pid_file = cfg.pid_file.clone();
    let service = Service::from_config(cfg)?;
    let result = rt.block_on(service.run());
    let _ = fs::remove_file(&pid_file);
    result
}

fn spawn_worker(args: &StartArgs) -> Result<Child> {
    let exe = std::env::current_exe().context("current executable path")?;
    let mut cmd = Command::new(exe);
    cmd.arg("__daemon");
    if let Some(bind) = &args.bind {
        cmd.arg("--bind").arg(bind);
    }
    if let Some(secs) = args.poll_secs {
        cmd.arg("--poll-secs").arg(secs.to_string());
    }
    if let Some(path) = &args.token_file {
        cmd.arg("--token-file").arg(path);
    } else if std::env::var("GGOK_TOKEN")
        .ok()
        .is_none_or(|s| s.is_empty())
    {
        if let Ok(path) = config::default_token_file() {
            if path.is_file() {
                cmd.arg("--token-file").arg(path);
            }
        }
    }
    if let Some(path) = &args.grok_home {
        cmd.arg("--grok-home").arg(path);
    }
    if let Some(bin) = &args.grok_bin {
        cmd.arg("--grok-bin").arg(bin);
    }
    if let Some(mode) = &args.permission_mode {
        cmd.arg("--permission-mode").arg(mode);
    }
    if let Some(n) = args.upload_max_bytes {
        cmd.arg("--upload-max-bytes").arg(n.to_string());
    }
    if let Some(path) = &args.config {
        cmd.arg("--config").arg(path);
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.spawn().context("spawn ggok __daemon")
}

fn wait_for_start(cfg: &RuntimeConfig, mut child: Child) -> Result<u32> {
    for _ in 0..20 {
        if let Some(pid) = running_pid(&cfg.pid_file) {
            let _ = child.try_wait();
            return Ok(pid);
        }
        if let Some(status) = child.try_wait().context("wait daemon worker")? {
            if let Some(pid) = running_pid(&cfg.pid_file) {
                return Ok(pid);
            }
            bail!(
                "failed to start (worker {status}); log {}",
                cfg.log_file.display()
            );
        }
        thread::sleep(Duration::from_millis(100));
    }
    if let Some(pid) = running_pid(&cfg.pid_file) {
        let _ = child.try_wait();
        return Ok(pid);
    }
    let _ = child.kill();
    bail!("failed to start; log {}", cfg.log_file.display())
}

fn enter_daemon(cfg: &RuntimeConfig) -> Result<()> {
    if let Some(pid) = running_pid(&cfg.pid_file) {
        bail!(
            "already running (pid {pid}, pid file {})",
            cfg.pid_file.display()
        );
    }
    if cfg.pid_file.exists() {
        fs::remove_file(&cfg.pid_file)
            .with_context(|| format!("remove stale pid file {}", cfg.pid_file.display()))?;
    }
    if let Some(dir) = cfg.pid_file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    if let Some(dir) = cfg.log_file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let log = File::options()
        .create(true)
        .append(true)
        .open(&cfg.log_file)
        .with_context(|| format!("open log {}", cfg.log_file.display()))?;
    Daemonize::new()
        .pid_file(&cfg.pid_file)
        .stdout(log.try_clone().context("clone log fd")?)
        .stderr(log)
        .working_directory("/")
        .umask(0o027)
        .start()
        .context("daemonize (fork)")?;
    Ok(())
}

fn print_report(kind: &str, pid: Option<u32>, bind: &str, log: &Path, grok_home: &Path) {
    match pid {
        Some(pid) => println!("{kind} pid={pid}"),
        None => println!("{kind}"),
    }
    println!("listen {bind}");
    println!("log {}", log.display());
    println!("sessions {}", grok_home.display());
    println!("login token: {}", display_token());
}

fn signal(pid: u32, sig: &str) {
    let _ = Command::new("kill")
        .arg(format!("-{sig}"))
        .arg(pid.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn kill_stale_agent() {
    let Ok(path) = agent_pid_file() else {
        return;
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return;
    };
    if let Ok(pid) = raw.trim().parse::<u32>() {
        signal(pid, "TERM");
        thread::sleep(Duration::from_millis(200));
        if pid_is_alive(pid) {
            signal(pid, "KILL");
        }
    }
    let _ = fs::remove_file(&path);
}
