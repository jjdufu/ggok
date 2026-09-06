use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::{self, IsTerminal};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

pub const DEFAULT_REPO: &str = "jjdufu/ggok";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const OS_LINUX: &str = "linux";
const OS_DARWIN: &str = "darwin";
const ARCH_AMD64: &str = "amd64";
const ARCH_ARM64: &str = "arm64";

/// Parsed `major.minor.patch` with optional pre-release suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => left.cmp(right),
            })
    }
}

/// # Errors
/// Returns an error if `raw` is not `owner/name` with each segment matching
/// `[A-Za-z0-9._-]+`, or if a segment is `.` / `..`, or if it ends in `.git`.
pub fn parse_repo(raw: &str) -> Result<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("repo is empty");
    }
    if Path::new(raw)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("git"))
    {
        bail!("repo must not end with .git");
    }
    let mut parts = raw.split('/');
    let Some(owner) = parts.next() else {
        bail!("repo must be owner/name");
    };
    let Some(name) = parts.next() else {
        bail!("repo must be owner/name");
    };
    if parts.next().is_some() {
        bail!("repo must be owner/name");
    }
    if !valid_repo_segment(owner) || !valid_repo_segment(name) {
        bail!("invalid repo {raw}");
    }
    Ok(format!("{owner}/{name}"))
}

fn valid_repo_segment(seg: &str) -> bool {
    if seg.is_empty() || seg == "." || seg == ".." {
        return false;
    }
    seg.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// # Errors
/// Returns an error if `GGOK_REPO` is set to a value that fails [`parse_repo`].
pub fn repo() -> Result<String> {
    let from_env = std::env::var("GGOK_REPO").ok();
    let raw = from_env
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_REPO);
    parse_repo(raw)
}

/// # Errors
/// Returns an error if [`repo`] fails.
pub fn latest_page_url() -> Result<String> {
    Ok(format!("https://github.com/{}/releases/latest", repo()?))
}

/// # Errors
/// Returns an error if [`repo`] fails, `ver` is not a version, or `os` / `arch`
/// are not in the allow-list.
pub fn asset_url(ver: &str, os: &str, arch: &str) -> Result<String> {
    let ver = version_string(&parse_version(ver)?);
    let (os, arch) = checked_os_arch(os, arch)?;
    Ok(format!(
        "https://github.com/{}/releases/download/v{ver}/{}",
        repo()?,
        asset_filename(&ver, os, arch)
    ))
}

/// # Errors
/// Returns an error if [`repo`] fails or `ver` is not a version.
pub fn sha256sums_url(ver: &str) -> Result<String> {
    let ver = version_string(&parse_version(ver)?);
    Ok(format!(
        "https://github.com/{}/releases/download/v{ver}/SHA256SUMS",
        repo()?
    ))
}

/// Filename of a release archive. Callers must pass values from [`os_arch`].
#[must_use]
pub fn asset_filename(ver: &str, os: &str, arch: &str) -> String {
    format!("ggok_{ver}_{os}_{arch}.tar.gz")
}

fn checked_os_arch<'a>(os: &'a str, arch: &'a str) -> Result<(&'a str, &'a str)> {
    if os != OS_LINUX && os != OS_DARWIN {
        bail!("unsupported OS: {os}");
    }
    if arch != ARCH_AMD64 && arch != ARCH_ARM64 {
        bail!("unsupported arch: {arch}");
    }
    Ok((os, arch))
}

/// # Errors
/// Returns an error if this binary's `OS` / `ARCH` is not linux/macos +
/// `x86_64`/`aarch64`.
pub fn os_arch() -> Result<(String, String)> {
    let os = match std::env::consts::OS {
        "linux" => OS_LINUX,
        "macos" => OS_DARWIN,
        other => bail!("unsupported OS: {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => ARCH_AMD64,
        "aarch64" => ARCH_ARM64,
        other => bail!("unsupported arch: {other}"),
    };
    Ok((os.to_string(), arch.to_string()))
}

/// # Errors
/// Returns an error if `raw` is empty, `latest`, missing a segment, contains
/// build metadata, or is not `major.minor.patch` with an optional pre suffix.
pub fn parse_version(raw: &str) -> Result<Version> {
    let trimmed = raw.trim();
    let rest = strip_one_v(trimmed);
    if rest.is_empty() || rest == "latest" || rest.contains('+') {
        bail!("invalid version {raw}");
    }
    let (core, pre) = match rest.split_once('-') {
        Some((_, "")) => bail!("invalid version {raw}"),
        Some((c, p)) => (c, Some(p.to_string())),
        None => (rest, None),
    };
    let mut segs = core.split('.');
    let Some(major) = parse_ver_num(segs.next()) else {
        bail!("invalid version {raw}");
    };
    let Some(minor) = parse_ver_num(segs.next()) else {
        bail!("invalid version {raw}");
    };
    let Some(patch) = parse_ver_num(segs.next()) else {
        bail!("invalid version {raw}");
    };
    if segs.next().is_some() {
        bail!("invalid version {raw}");
    }
    Ok(Version {
        major,
        minor,
        patch,
        pre,
    })
}

fn parse_ver_num(seg: Option<&str>) -> Option<u64> {
    let seg = seg?;
    if seg.is_empty() || !seg.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    seg.parse().ok()
}

fn strip_one_v(s: &str) -> &str {
    s.strip_prefix('v')
        .or_else(|| s.strip_prefix('V'))
        .unwrap_or(s)
}

fn version_string(ver: &Version) -> String {
    match &ver.pre {
        Some(pre) => format!("{}.{}.{}-{pre}", ver.major, ver.minor, ver.patch),
        None => format!("{}.{}.{}", ver.major, ver.minor, ver.patch),
    }
}

/// Whether `latest` should be treated as an update over `current`.
///
/// Pre-release `latest` is never an update when `current` is a stable tag.
/// Both pre-release suffixes compare as strings (so `rc.10` < `rc.9`).
#[must_use]
pub fn is_newer(latest: &str, current: &str) -> bool {
    let Ok(latest_v) = parse_version(latest) else {
        return false;
    };
    let Ok(current_v) = parse_version(current) else {
        return false;
    };
    if latest_v.pre.is_some() && current_v.pre.is_none() {
        return false;
    }
    latest_v > current_v
}

/// # Errors
/// Returns an error if `effective_url` is empty, ends at `latest`, or the last
/// path segment is not a version tag.
pub fn parse_latest_tag(effective_url: &str) -> Result<String> {
    let trimmed = effective_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("empty latest URL");
    }
    let tag = trimmed.rsplit('/').next().unwrap_or("");
    if tag.is_empty() || tag == "latest" {
        bail!("not a release tag URL");
    }
    let ver = parse_version(tag)?;
    Ok(version_string(&ver))
}

/// # Errors
/// Returns an error if no line matches `filename` exactly, or the matching hex
/// is not 64 hexadecimal characters.
pub fn parse_sha256sums(text: &str, filename: &str) -> Result<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(hex) = parts.next() else {
            continue;
        };
        let Some(name) = parts.next() else {
            continue;
        };
        if parts.next().is_some() {
            continue;
        }
        let name = name.strip_prefix('*').unwrap_or(name);
        if name != filename {
            continue;
        }
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("invalid sha256 hex for {filename}");
        }
        return Ok(hex.to_string());
    }
    bail!("no sha256 for {filename}");
}

/// # Errors
/// Returns an error if `path` cannot be read, `hex` is not valid hexadecimal,
/// or the digest does not match.
pub fn verify_file_sha256(path: &Path, hex: &str) -> Result<()> {
    let expected = hex::decode(hex.trim()).context("decode sha256 hex")?;
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).with_context(|| format!("read {}", path.display()))?;
    let got = hasher.finalize();
    if got.as_slice() == expected.as_slice() {
        Ok(())
    } else {
        bail!("sha256 mismatch for {}", path.display());
    }
}

/// # Errors
/// Returns an error if `curl` fails or the effective URL cannot be read.
pub fn curl_effective_url(url: &str) -> Result<String> {
    let output = curl_cmd()
        .args([
            "-fsSL",
            "--retry",
            "3",
            "--connect-timeout",
            "4",
            "--max-time",
            "8",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            url,
        ])
        .output()
        .with_context(|| format!("run {} for effective URL", curl_bin()))?;
    check_curl(&output, url)?;
    String::from_utf8(output.stdout).context("curl effective URL is not utf-8")
}

/// # Errors
/// Returns an error if `curl` fails or `dest` cannot be written.
///
/// When stderr is a terminal, uses curl's `--progress-bar` so the bar tracks
/// real downloaded bytes. Otherwise the download is silent.
pub fn curl_download(url: &str, dest: &Path) -> Result<()> {
    let progress = io::stderr().is_terminal();
    let mut cmd = curl_cmd();
    cmd.args([
        "-fL",
        "--retry",
        "3",
        "--connect-timeout",
        "4",
        "--max-time",
        "120",
    ]);
    if progress {
        cmd.arg("--progress-bar");
    } else {
        cmd.args(["-sS"]);
    }
    cmd.arg("-o").arg(dest).arg(url);
    if progress {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::inherit());
        let status = cmd
            .status()
            .with_context(|| format!("run {} download {}", curl_bin(), dest.display()))?;
        if status.success() {
            eprintln!();
            Ok(())
        } else {
            bail!("curl failed for {url} ({status})")
        }
    } else {
        let output = cmd
            .output()
            .with_context(|| format!("run {} download {}", curl_bin(), dest.display()))?;
        check_curl(&output, url)
    }
}

/// # Errors
/// Returns an error if `curl` fails or the body is not UTF-8.
pub fn curl_to_string(url: &str) -> Result<String> {
    let output = curl_cmd()
        .args([
            "-fsSL",
            "--retry",
            "3",
            "--connect-timeout",
            "4",
            "--max-time",
            "8",
            url,
        ])
        .output()
        .with_context(|| format!("run {} for {url}", curl_bin()))?;
    check_curl(&output, url)?;
    String::from_utf8(output.stdout).context("curl body is not utf-8")
}

/// # Errors
/// Returns an error if [`latest_page_url`], `curl`, or [`parse_latest_tag`] fails.
pub fn fetch_latest_version() -> Result<String> {
    parse_latest_tag(&curl_effective_url(&latest_page_url()?)?)
}

fn curl_bin() -> &'static str {
    if Path::new("/usr/bin/curl").is_file() {
        "/usr/bin/curl"
    } else {
        "curl"
    }
}

fn curl_cmd() -> Command {
    let mut cmd = Command::new(curl_bin());
    cmd.stdin(Stdio::null());
    cmd
}

fn check_curl(output: &std::process::Output, url: &str) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr);
    let err = err.trim();
    if err.is_empty() {
        bail!("curl failed for {url} ({})", output.status);
    }
    bail!("curl failed for {url}: {err}");
}

/// Replace `dest` with `src` using same-directory rename (never truncate `dest`).
///
/// # Errors
/// Returns an error if a filesystem step fails. On failure, a previous `dest`
/// is restored when a backup exists, and `.ggok.new` is removed.
pub fn replace_file_atomic(src: &Path, dest: &Path) -> Result<()> {
    let parent = dest
        .parent()
        .ok_or_else(|| anyhow!("destination has no parent: {}", dest.display()))?;
    let tmp_installed = parent.join(".ggok.new");
    let backup = parent.join(".ggok.old");
    let result = replace_file_atomic_inner(src, dest, parent, &tmp_installed, &backup);
    if result.is_err() {
        if backup.exists()
            && !dest.exists()
            && let Err(e) = fs::rename(&backup, dest)
        {
            eprintln!("restore {} → {}: {e}", backup.display(), dest.display());
        }
        if let Err(e) = remove_existing(&tmp_installed) {
            eprintln!("remove {}: {e}", tmp_installed.display());
        }
    }
    result
}

fn replace_file_atomic_inner(
    src: &Path,
    dest: &Path,
    parent: &Path,
    tmp_installed: &Path,
    backup: &Path,
) -> Result<()> {
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    remove_existing(tmp_installed)?;
    if !dest.exists() && backup.exists() {
        fs::rename(backup, dest)
            .with_context(|| format!("restore {} → {}", backup.display(), dest.display()))?;
    }
    fs::copy(src, tmp_installed)
        .with_context(|| format!("copy {} → {}", src.display(), tmp_installed.display()))?;
    fs::set_permissions(tmp_installed, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("chmod {}", tmp_installed.display()))?;
    if dest.exists() {
        fs::rename(dest, backup)
            .with_context(|| format!("rename {} → {}", dest.display(), backup.display()))?;
    }
    fs::rename(tmp_installed, dest)
        .with_context(|| format!("rename {} → {}", tmp_installed.display(), dest.display()))?;
    if let Err(e) = fs::remove_file(backup)
        && e.kind() != io::ErrorKind::NotFound
    {
        eprintln!("remove {}: {e}", backup.display());
    }
    Ok(())
}

fn remove_existing(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(anyhow!(e).context(format!("stat {}", path.display()))),
        Ok(meta) if meta.is_dir() => {
            fs::remove_dir_all(path).with_context(|| format!("remove {}", path.display()))
        }
        Ok(_) => fs::remove_file(path).with_context(|| format!("remove {}", path.display())),
    }
}
