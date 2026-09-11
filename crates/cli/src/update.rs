use crate::ctl::{is_build_artifact, stop_web};
use anyhow::{Context, Result, anyhow, bail};
use ggok_core::config::{log_file, pid_file, running_pid};
use ggok_core::paths::is_under;
use ggok_core::release::{
    CURRENT_VERSION, asset_filename, asset_url, fetch_latest_version, is_newer, os_arch,
    parse_sha256sums, release_asset_ready, replace_file_atomic, sha256sums_url, verify_file_sha256,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TmpDir(PathBuf);

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// # Errors
/// Returns an error if dest cannot be resolved, latest cannot be fetched, the
/// archive fails verification, or replace / restart fails.
pub(crate) fn run() -> Result<i32> {
    let dest = resolve_dest()?;
    let current = CURRENT_VERSION;
    let latest = fetch_latest_version().map_err(|_| anyhow!("Could not check for updates"))?;
    if !is_newer(&latest, current) {
        println!("Already up to date ({current})");
        return Ok(0);
    }
    let (os, arch) = os_arch()?;
    if !release_asset_ready(&latest, &os, &arch)? {
        bail!("Release {latest} is not ready yet");
    }
    let filename = asset_filename(&latest, &os, &arch);
    let tmp = make_tmp()?;
    println!("Updating {current} → {latest}");
    println!("Downloading");
    let archive = tmp.0.join(&filename);
    curl_and_verify(&latest, &os, &arch, &filename, &archive)?;
    println!("Installing");
    let extracted = extract_ggok(&archive, &tmp.0)?;
    replace_file_atomic(&extracted, &dest)
        .map_err(|_| anyhow!("Could not replace the ggok binary"))?;
    println!("Updated to {latest}");
    restart_web_if_running(&dest, &latest)?;
    Ok(0)
}

fn curl_and_verify(
    latest: &str,
    os: &str,
    arch: &str,
    filename: &str,
    archive: &Path,
) -> Result<()> {
    ggok_core::release::curl_download(&asset_url(latest, os, arch)?, archive)
        .map_err(|e| annotate_download(&e, latest))?;
    println!("Verifying");
    let sums = ggok_core::release::curl_to_string(&sha256sums_url(latest)?)
        .map_err(|_| anyhow!("Could not check for updates"))?;
    let hex = parse_sha256sums(&sums, filename)
        .map_err(|_| anyhow!("Download was corrupted\nTry ggok update again"))?;
    verify_file_sha256(archive, &hex)
        .map_err(|_| anyhow!("Download was corrupted\nTry ggok update again"))
}

fn annotate_download(err: &anyhow::Error, latest: &str) -> anyhow::Error {
    match err.to_string().as_str() {
        "Network timeout while downloading" => anyhow!(
            "Network timeout while downloading {latest}\nCheck the connection and run ggok update again"
        ),
        "Could not download update" => {
            anyhow!("Could not download update\nCheck the connection and run ggok update again")
        }
        "Release is not ready yet" => anyhow!("Release {latest} is not ready yet"),
        other => anyhow!("{other}"),
    }
}

fn resolve_dest() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if is_build_artifact(&exe) {
            bail!("Refusing to update a cargo build");
        }
        if exe.file_name().and_then(|n| n.to_str()) == Some("ggok") {
            return Ok(exe);
        }
    }
    let home = std::env::var("HOME").context("HOME is not set")?;
    if home.is_empty() {
        bail!("HOME is not set");
    }
    Ok(PathBuf::from(home).join(".local/bin/ggok"))
}

fn make_tmp() -> Result<TmpDir> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!("ggok-update-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).map_err(|_| anyhow!("Could not replace the ggok binary"))?;
    Ok(TmpDir(path))
}

fn extract_ggok(archive: &Path, tmp: &Path) -> Result<PathBuf> {
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(tmp)
        .stdin(Stdio::null())
        .status()
        .map_err(|_| anyhow!("Update archive is invalid"))?;
    if !status.success() {
        bail!("Update archive is invalid");
    }
    let bin = tmp.join("ggok");
    let meta = fs::symlink_metadata(&bin).map_err(|_| anyhow!("Update archive is invalid"))?;
    if !meta.file_type().is_file() {
        bail!("Update archive is invalid");
    }
    let canon_bin = fs::canonicalize(&bin).map_err(|_| anyhow!("Update archive is invalid"))?;
    let canon_tmp = fs::canonicalize(tmp).map_err(|_| anyhow!("Update archive is invalid"))?;
    if !is_under(&canon_bin, &canon_tmp) {
        bail!("Update archive is invalid");
    }
    Ok(bin)
}

fn restart_web_if_running(dest: &Path, latest: &str) -> Result<()> {
    let running = pid_file()
        .ok()
        .and_then(|path| running_pid(&path))
        .is_some();
    if !running {
        return Ok(());
    }
    stop_web(false)?;
    let status = Command::new(dest)
        .arg("start")
        .status()
        .map_err(|_| restart_failed(latest))?;
    if status.success() {
        println!("Restarted");
        Ok(())
    } else {
        Err(restart_failed(latest))
    }
}

fn restart_failed(latest: &str) -> anyhow::Error {
    let log = log_file().map_or_else(|_| "the log".to_string(), |p| p.display().to_string());
    anyhow!("Updated to {latest} but could not restart\nSee {log}")
}
