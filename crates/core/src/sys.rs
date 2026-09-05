use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn has_proc() -> bool {
    Path::new("/proc").is_dir()
}

#[must_use]
pub fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    if has_proc() {
        return Path::new(&format!("/proc/{pid}")).is_dir();
    }
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[must_use]
pub fn pid_cmdline(pid: u32) -> Vec<u8> {
    if has_proc() {
        return std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    }
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "args="])
        .output()
        .map(|o| o.stdout)
        .unwrap_or_default()
}

#[must_use]
pub fn effective_uid() -> Option<u32> {
    if has_proc() {
        let raw = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                return rest.split_whitespace().nth(1)?.parse().ok();
            }
        }
        return None;
    }
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
}

#[must_use]
pub fn resolve_default_grok_bin() -> PathBuf {
    if has_proc() {
        return PathBuf::from("grok");
    }
    discover_grok_bin().unwrap_or_else(|| PathBuf::from("grok"))
}

fn discover_grok_bin() -> Option<PathBuf> {
    if let Some(p) = which_bin("grok") {
        return Some(p);
    }
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        cands.push(home.join(".grok/bin/grok"));
        cands.push(home.join(".local/bin/grok"));
        cands.push(home.join(".cargo/bin/grok"));
    }
    if let Ok(gh) = std::env::var("GROK_HOME") {
        let gh = PathBuf::from(gh);
        cands.push(gh.join("bin/grok"));
        cands.push(gh.join("grok"));
    }
    cands.push(PathBuf::from("/opt/homebrew/bin/grok"));
    cands.push(PathBuf::from("/usr/local/bin/grok"));
    cands.into_iter().find(|p| p.is_file())
}

fn which_bin(name: &str) -> Option<PathBuf> {
    let out = Command::new("which")
        .arg(name)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8(out.stdout).ok()?;
    let p = p.trim();
    if p.is_empty() {
        return None;
    }
    let pb = PathBuf::from(p);
    pb.is_file().then_some(pb)
}
