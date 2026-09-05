use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const DEFAULT_BIND: &str = "0.0.0.0:9888";

#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub bind: Option<String>,
    pub token_file: Option<PathBuf>,
    pub grok_home: Option<PathBuf>,
    pub grok_bin: Option<String>,
    pub poll_secs: Option<u64>,
    pub permission_mode: Option<String>,
    pub upload_max_bytes: Option<u64>,
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub bind: String,
    pub token: String,
    pub cookie_key: [u8; 32],
    pub grok_home: PathBuf,
    pub grok_bin: PathBuf,
    pub poll_secs: u64,
    pub permission_mode: String,
    pub upload_max_bytes: u64,
    pub workspace_roots: Vec<PathBuf>,
    pub pid_file: PathBuf,
    pub log_file: PathBuf,
    pub state_file: PathBuf,
    pub agent_pid_file: PathBuf,
    pub leader_json_file: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    bind: Option<String>,
    #[serde(default)]
    token_file: Option<PathBuf>,
    #[serde(default)]
    grok_home: Option<PathBuf>,
    #[serde(default)]
    grok_bin: Option<String>,
    #[serde(default)]
    poll_secs: Option<u64>,
    #[serde(default)]
    permission_mode: Option<String>,
    #[serde(default)]
    upload_max_bytes: Option<u64>,
    #[serde(default)]
    workspace_roots: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedState {
    pub bind: String,
    pub grok_home: PathBuf,
    pub log_file: PathBuf,
}

impl RuntimeConfig {
    /// # Errors
    /// Returns an error if the token, grok binary, or config file cannot be prepared.
    pub fn prepare(overrides: ConfigOverrides) -> Result<Self> {
        build(overrides, true)
    }

    /// # Errors
    /// Returns an error if the token, grok binary, or config file cannot be read.
    pub fn from_overrides(overrides: ConfigOverrides) -> Result<Self> {
        build(overrides, false)
    }

    /// # Errors
    /// Returns an error if the state directory cannot be created or the file cannot be written.
    pub fn write_saved_state(&self) -> Result<()> {
        let state = SavedState {
            bind: self.bind.clone(),
            grok_home: self.grok_home.clone(),
            log_file: self.log_file.clone(),
        };
        if let Some(dir) = self.state_file.parent() {
            fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        let json = serde_json::to_vec_pretty(&state).context("serialize state")?;
        fs::write(&self.state_file, json)
            .with_context(|| format!("write {}", self.state_file.display()))?;
        Ok(())
    }
}

fn build(overrides: ConfigOverrides, create_token: bool) -> Result<RuntimeConfig> {
    let file = load_file_config(overrides.config.as_ref())?;
    let token_file = overrides.token_file.or(file.token_file);
    let token = if create_token {
        prepare_token(token_file.as_ref())?
    } else {
        load_token(token_file.as_ref())?
    };
    if token.is_empty() {
        bail!("token is empty; refuse to start");
    }
    let grok_home = resolve_grok_home(overrides.grok_home.or(file.grok_home))?;
    let grok_bin = resolve_grok_bin(overrides.grok_bin.or(file.grok_bin))?;
    confirm_grok_bin(&grok_bin)?;
    let cookie_key = derive_cookie_key(&token);
    let poll_secs = overrides.poll_secs.or(file.poll_secs).unwrap_or(5);
    let poll_secs = if poll_secs == 0 { 5 } else { poll_secs };
    let permission_mode = normalize_permission_mode(
        overrides
            .permission_mode
            .or(file.permission_mode)
            .as_deref()
            .unwrap_or("ask"),
    )?;
    let upload_max_bytes = overrides
        .upload_max_bytes
        .or(file.upload_max_bytes)
        .unwrap_or(20 * 1024 * 1024);
    if upload_max_bytes == 0 {
        bail!("upload_max_bytes must be > 0");
    }
    let workspace_roots = crate::paths::expand_roots(file.workspace_roots.as_deref());
    let bind = overrides
        .bind
        .or(file.bind)
        .unwrap_or_else(|| DEFAULT_BIND.to_string());
    let state = state_dir()?;
    Ok(RuntimeConfig {
        bind,
        token,
        cookie_key,
        grok_home,
        grok_bin,
        poll_secs,
        permission_mode,
        upload_max_bytes,
        workspace_roots,
        pid_file: state.join("ggok.pid"),
        log_file: state.join("ggok.log"),
        state_file: state.join("state.json"),
        agent_pid_file: state.join("grok-agent.pid"),
        leader_json_file: state.join("grok-leader.json"),
    })
}

fn load_file_config(explicit: Option<&PathBuf>) -> Result<FileConfig> {
    let path = if let Some(p) = explicit {
        p.clone()
    } else if let Some(p) = env_nonempty("GGOK_CONFIG") {
        PathBuf::from(p)
    } else {
        config_dir()?.join("config.toml")
    };
    if !path.is_file() {
        return Ok(FileConfig::default());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn resolve_grok_bin(explicit: Option<String>) -> Result<PathBuf> {
    if let Some(raw) = explicit {
        if raw.is_empty() {
            bail!("grok_bin is empty");
        }
        return Ok(PathBuf::from(raw));
    }
    Ok(crate::sys::resolve_default_grok_bin())
}

fn confirm_grok_bin(bin: &Path) -> Result<()> {
    let status = std::process::Command::new(bin)
        .arg("agent")
        .arg("stdio")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => bail!("grok_bin {} agent stdio --help exited {}", bin.display(), s),
        Err(e) => bail!("grok_bin {} is not runnable: {e}", bin.display()),
    }
}

fn normalize_permission_mode(raw: &str) -> Result<String> {
    match raw.trim() {
        "ask" => Ok("ask".to_string()),
        "auto" => Ok("auto".to_string()),
        "always-approve" => Ok("always-approve".to_string()),
        other => bail!("permission_mode must be ask, auto, or always-approve (got {other})"),
    }
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

fn user_home() -> Result<PathBuf> {
    env_nonempty("HOME")
        .map(PathBuf::from)
        .context("HOME is unset")
}

/// # Errors
/// Returns an error if `HOME` is unset and `XDG_CONFIG_HOME` is not set.
pub fn config_dir() -> Result<PathBuf> {
    if let Some(xdg) = env_nonempty("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("ggok"));
    }
    Ok(user_home()?.join(".config/ggok"))
}

/// # Errors
/// Returns an error if `HOME` is unset and `XDG_STATE_HOME` is not set.
pub fn state_dir() -> Result<PathBuf> {
    if let Some(xdg) = env_nonempty("XDG_STATE_HOME") {
        return Ok(PathBuf::from(xdg).join("ggok"));
    }
    Ok(user_home()?.join(".local/state/ggok"))
}

/// # Errors
/// Returns an error if the config directory cannot be resolved.
pub fn default_token_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("token"))
}

/// # Errors
/// Returns an error if the state directory cannot be resolved.
pub fn pid_file() -> Result<PathBuf> {
    Ok(state_dir()?.join("ggok.pid"))
}

/// # Errors
/// Returns an error if the state directory cannot be resolved.
pub fn log_file() -> Result<PathBuf> {
    Ok(state_dir()?.join("ggok.log"))
}

/// # Errors
/// Returns an error if the state directory cannot be resolved.
pub fn state_file() -> Result<PathBuf> {
    Ok(state_dir()?.join("state.json"))
}

/// # Errors
/// Returns an error if the state directory cannot be resolved.
pub fn agent_pid_file() -> Result<PathBuf> {
    Ok(state_dir()?.join("grok-agent.pid"))
}

/// # Errors
/// Returns an error if the state directory cannot be resolved.
pub fn leader_json_file() -> Result<PathBuf> {
    Ok(state_dir()?.join("grok-leader.json"))
}

fn resolve_grok_home(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        if p.as_os_str().is_empty() {
            bail!("--grok-home is empty");
        }
        return Ok(p);
    }
    let home = user_home().context("HOME is unset; pass --grok-home or set GROK_HOME")?;
    Ok(home.join(".grok"))
}

/// # Errors
/// Returns an error if `HOME` is unset.
pub fn default_grok_home() -> Result<PathBuf> {
    resolve_grok_home(None)
}

fn load_token(token_file: Option<&PathBuf>) -> Result<String> {
    if let Some(path) = token_file {
        return read_token_file(path);
    }
    if let Some(token) = env_nonempty("GGOK_TOKEN") {
        return Ok(token);
    }
    let default = default_token_file()?;
    if default.is_file() {
        return read_token_file(&default);
    }
    bail!(
        "no token file at {} and GGOK_TOKEN unset",
        default.display()
    )
}

/// # Errors
/// Returns an error if the token file cannot be read or created, or permissions are not `600`.
pub fn prepare_token(token_file: Option<&PathBuf>) -> Result<String> {
    if let Some(path) = token_file {
        if path.is_file() {
            return read_token_file(path);
        }
        let token = random_token()?;
        return write_token_file(path, &token);
    }
    if let Some(token) = env_nonempty("GGOK_TOKEN") {
        let default = default_token_file()?;
        if !default.is_file() {
            write_token_file(&default, &token)?;
        }
        return Ok(token);
    }
    let default = default_token_file()?;
    if default.is_file() {
        return read_token_file(&default);
    }
    let token = random_token()?;
    write_token_file(&default, &token)
}

#[must_use]
pub fn display_token() -> String {
    if let Ok(path) = default_token_file()
        && let Ok(raw) = fs::read_to_string(&path)
    {
        let token = raw.trim();
        if !token.is_empty() {
            return token.to_string();
        }
    }
    if let Some(token) = env_nonempty("GGOK_TOKEN") {
        return token;
    }
    match default_token_file() {
        Ok(path) => format!("(missing {})", path.display()),
        Err(_) => "(missing)".to_string(),
    }
}

#[must_use]
pub fn read_saved_state() -> Option<SavedState> {
    let path = state_file().ok()?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn random_token() -> Result<String> {
    let mut buf = [0_u8; 24];
    File::open("/dev/urandom")
        .context("open /dev/urandom")?
        .read_exact(&mut buf)
        .context("read /dev/urandom")?;
    Ok(hex::encode(buf))
}

fn write_token_file(path: &Path, token: &str) -> Result<String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(mut f) => {
            writeln!(f, "{token}").with_context(|| format!("write token {}", path.display()))?;
            f.sync_all()
                .with_context(|| format!("sync token {}", path.display()))?;
            Ok(token.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => read_token_file(path),
        Err(e) => Err(e).with_context(|| format!("create token file {}", path.display())),
    }
}

fn read_token_file(path: &Path) -> Result<String> {
    let meta =
        fs::metadata(path).with_context(|| format!("token file missing: {}", path.display()))?;
    let mode = meta.permissions().mode();
    if mode & 0o077 != 0 {
        bail!(
            "token file {} permissions must be 600 (got {:o})",
            path.display(),
            mode & 0o777
        );
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("read token file {}", path.display()))?;
    let token = raw.trim().to_string();
    if token.is_empty() {
        bail!("token file {} is empty", path.display());
    }
    Ok(token)
}

#[must_use]
pub fn running_pid(pid_file: &Path) -> Option<u32> {
    let raw = fs::read_to_string(pid_file).ok()?;
    let pid: u32 = raw.trim().parse().ok()?;
    if crate::sys::pid_is_alive(pid) {
        Some(pid)
    } else {
        None
    }
}

fn derive_cookie_key(token: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"ggok-cookie-v1");
    hasher.update(token.as_bytes());
    hasher.finalize().into()
}
