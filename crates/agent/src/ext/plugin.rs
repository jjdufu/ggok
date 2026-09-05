use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const LIST_TIMEOUT: Duration = Duration::from_secs(60);
const MUTATE_TIMEOUT: Duration = Duration::from_secs(180);

/// # Errors
/// Returns an error if a grok `plugin` list subprocess fails.
pub async fn snapshot(bin: &Path, cwd: &Path) -> Result<Value> {
    let plugins = run_json(
        bin,
        cwd,
        &["plugin", "list", "--json", "--available"],
        LIST_TIMEOUT,
    )
    .await?;
    let sources = run_json(bin, cwd, &["plugin", "marketplace", "list", "--json"], LIST_TIMEOUT).await?;
    Ok(json!({
        "plugins": coerce_list(plugins),
        "sources": coerce_list(sources),
    }))
}

/// # Errors
/// Returns an error if `source` is empty or grok `plugin install` fails.
pub async fn install(bin: &Path, cwd: &Path, source: &str) -> Result<String> {
    check_source(source)?;
    run_text(
        bin,
        cwd,
        &["plugin".into(), "install".into(), "--trust".into(), source.trim().into()],
        MUTATE_TIMEOUT,
    )
    .await
}

/// # Errors
/// Returns an error if the name is invalid or grok `plugin uninstall` fails.
pub async fn uninstall(bin: &Path, cwd: &Path, name: &str) -> Result<String> {
    check_name(name)?;
    run_text(
        bin,
        cwd,
        &[
            "plugin".into(),
            "uninstall".into(),
            "--confirm".into(),
            name.trim().into(),
        ],
        MUTATE_TIMEOUT,
    )
    .await
}

/// # Errors
/// Returns an error if the name is invalid or grok `plugin enable` fails.
pub async fn enable(bin: &Path, cwd: &Path, name: &str) -> Result<String> {
    check_name(name)?;
    run_text(
        bin,
        cwd,
        &["plugin".into(), "enable".into(), name.trim().into()],
        LIST_TIMEOUT,
    )
    .await
}

/// # Errors
/// Returns an error if the name is invalid or grok `plugin disable` fails.
pub async fn disable(bin: &Path, cwd: &Path, name: &str) -> Result<String> {
    check_name(name)?;
    run_text(
        bin,
        cwd,
        &["plugin".into(), "disable".into(), name.trim().into()],
        LIST_TIMEOUT,
    )
    .await
}

/// # Errors
/// Returns an error if the name is invalid or grok `plugin update` fails.
pub async fn update(bin: &Path, cwd: &Path, name: Option<&str>) -> Result<String> {
    let mut argv = vec!["plugin".to_string(), "update".to_string()];
    if let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) {
        check_name(name)?;
        argv.push(name.to_string());
    }
    run_text(bin, cwd, &argv, MUTATE_TIMEOUT).await
}

/// # Errors
/// Returns an error if `source` is empty or grok marketplace add fails.
pub async fn marketplace_add(bin: &Path, cwd: &Path, source: &str) -> Result<String> {
    check_source(source)?;
    run_text(
        bin,
        cwd,
        &[
            "plugin".into(),
            "marketplace".into(),
            "add".into(),
            source.trim().into(),
        ],
        MUTATE_TIMEOUT,
    )
    .await
}

/// # Errors
/// Returns an error if `source` is empty or grok marketplace remove fails.
pub async fn marketplace_remove(bin: &Path, cwd: &Path, source: &str) -> Result<String> {
    check_source(source)?;
    run_text(
        bin,
        cwd,
        &[
            "plugin".into(),
            "marketplace".into(),
            "remove".into(),
            source.trim().into(),
        ],
        MUTATE_TIMEOUT,
    )
    .await
}

/// # Errors
/// Returns an error if `source` is invalid or grok marketplace update fails.
pub async fn marketplace_update(bin: &Path, cwd: &Path, source: Option<&str>) -> Result<String> {
    let mut argv = vec![
        "plugin".to_string(),
        "marketplace".to_string(),
        "update".to_string(),
    ];
    if let Some(source) = source.map(str::trim).filter(|s| !s.is_empty()) {
        check_source(source)?;
        argv.push(source.to_string());
    }
    run_text(bin, cwd, &argv, MUTATE_TIMEOUT).await
}

fn check_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("name required");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        bail!("name may only contain letters, numbers, dots, hyphens, and underscores");
    }
    Ok(())
}

fn check_source(raw: &str) -> Result<()> {
    const BAD: &str = ";|&$`()<>\\\"'";
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("source required");
    }
    if raw.chars().any(char::is_control) {
        bail!("invalid source");
    }
    if raw.contains("..") {
        bail!("invalid source");
    }
    if raw.chars().any(|c| BAD.contains(c)) {
        bail!("invalid source");
    }
    Ok(())
}

fn coerce_list(v: Value) -> Value {
    if v.is_array() {
        return v;
    }
    if let Some(arr) = v.get("plugins").or_else(|| v.get("sources")).cloned() {
        if arr.is_array() {
            return arr;
        }
    }
    json!([])
}

async fn run_json(bin: &Path, cwd: &Path, args: &[&str], limit: Duration) -> Result<Value> {
    let text = run_raw(bin, cwd, args, limit).await?;
    if text.is_empty() {
        return Ok(json!([]));
    }
    match serde_json::from_str(&text) {
        Ok(v) => Ok(v),
        Err(_) => Ok(json!({ "text": text })),
    }
}

async fn run_text(bin: &Path, cwd: &Path, args: &[String], limit: Duration) -> Result<String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    run_raw(bin, cwd, &args, limit).await
}

async fn run_raw(bin: &Path, cwd: &Path, args: &[&str], limit: Duration) -> Result<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let out = timeout(limit, cmd.output())
        .await
        .context("grok plugin timed out")?
        .with_context(|| format!("spawn grok {}", args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        return Ok(if stdout.is_empty() { stderr } else { stdout });
    }
    let msg = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("grok plugin exited {}", out.status)
    };
    bail!("{msg}")
}
