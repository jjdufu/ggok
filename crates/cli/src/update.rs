use crate::ctl::{is_build_artifact, stop_web};
use anyhow::{Context, Result, bail};
use ggok_core::config::{pid_file, running_pid};
use ggok_core::paths::is_under;
use ggok_core::release::{
    CURRENT_VERSION, asset_filename, asset_url, fetch_latest_version, is_newer, os_arch,
    parse_sha256sums, replace_file_atomic, sha256sums_url, verify_file_sha256,
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
    let latest = fetch_latest_version()?;
    if !is_newer(&latest, current) {
        println!("already {current}");
        return Ok(0);
    }
    let (os, arch) = os_arch()?;
    let filename = asset_filename(&latest, &os, &arch);
    let tmp = make_tmp()?;
    println!("updating {current} → {latest}");
    println!("downloading {filename}");
    let archive = tmp.0.join(&filename);
    curl_and_verify(&latest, &os, &arch, &filename, &archive)?;
    println!("verified sha256");
    let extracted = extract_ggok(&archive, &tmp.0)?;
    replace_file_atomic(&extracted, &dest)?;
    println!("installed {}", dest.display());
    restart_web_if_running(&dest)?;
    Ok(0)
}

fn curl_and_verify(
    latest: &str,
    os: &str,
    arch: &str,
    filename: &str,
    archive: &Path,
) -> Result<()> {
    ggok_core::release::curl_download(&asset_url(latest, os, arch)?, archive)?;
    let sums = ggok_core::release::curl_to_string(&sha256sums_url(latest)?)?;
    let hex = parse_sha256sums(&sums, filename)?;
    verify_file_sha256(archive, &hex)
}

fn resolve_dest() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if is_build_artifact(&exe) {
            bail!("refusing to update a cargo build artifact");
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
    fs::create_dir_all(&path).with_context(|| format!("create {}", path.display()))?;
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
        .context("run tar")?;
    if !status.success() {
        bail!("tar failed with {status}");
    }
    let bin = tmp.join("ggok");
    let meta = fs::symlink_metadata(&bin)
        .with_context(|| format!("archive is missing ggok ({})", bin.display()))?;
    if !meta.file_type().is_file() {
        bail!("archive ggok is not a regular file");
    }
    let canon_bin =
        fs::canonicalize(&bin).with_context(|| format!("canonicalize {}", bin.display()))?;
    let canon_tmp =
        fs::canonicalize(tmp).with_context(|| format!("canonicalize {}", tmp.display()))?;
    if !is_under(&canon_bin, &canon_tmp) {
        bail!("archive ggok escapes the extract directory");
    }
    Ok(bin)
}

fn restart_web_if_running(dest: &Path) -> Result<()> {
    let running = pid_file()
        .ok()
        .and_then(|path| running_pid(&path))
        .is_some();
    if !running {
        return Ok(());
    }
    stop_web(true)?;
    let status = Command::new(dest)
        .arg("start")
        .status()
        .with_context(|| format!("start {}", dest.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!("ggok start failed with {status}");
    }
}
