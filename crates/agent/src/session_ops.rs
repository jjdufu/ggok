use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

const DELETE_TIMEOUT: Duration = Duration::from_secs(30);

/// # Errors
/// Returns an error if `id` is not a UUID or grok `sessions delete` fails.
pub async fn delete_session(bin: &Path, grok_home: &Path, id: &str) -> Result<()> {
    if Uuid::parse_str(id).is_err() {
        bail!("invalid session id");
    }
    let mut cmd = Command::new(bin);
    cmd.args(["sessions", "delete", id])
        .current_dir(grok_home)
        .env("GROK_HOME", grok_home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let out = timeout(DELETE_TIMEOUT, cmd.output())
        .await
        .context("grok sessions delete timed out")?
        .with_context(|| format!("spawn grok sessions delete {id}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let msg = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("grok sessions delete exited {}", out.status)
    };
    bail!("{msg}")
}
