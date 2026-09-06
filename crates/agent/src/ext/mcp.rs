use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const MCP_TIMEOUT: Duration = Duration::from_secs(45);

/// # Errors
/// Returns an error if the grok `mcp` subprocess fails.
pub async fn snapshot(bin: &Path, cwd: &Path) -> Result<Value> {
    let list_fut = run_json(bin, cwd, &["mcp", "list", "--json"]);
    let doctor_fut = run_json(bin, cwd, &["mcp", "doctor", "--json"]);
    let (list, doctor) = tokio::try_join!(list_fut, doctor_fut)?;
    Ok(merge(&list, &doctor))
}

pub struct AddSpec<'a> {
    pub name: &'a str,
    pub transport: &'a str,
    pub command_or_url: &'a str,
    pub args: &'a [String],
    pub scope: &'a str,
    pub env: &'a [String],
    pub headers: &'a [String],
}

/// # Errors
/// Returns an error if the name, transport, or scope is invalid, or grok `mcp add` fails.
pub async fn add(bin: &Path, cwd: &Path, spec: AddSpec<'_>) -> Result<String> {
    check_name(spec.name)?;
    let transport = normalize_transport(spec.transport)?;
    let scope = normalize_scope(spec.scope)?;
    let mut argv = vec!["mcp".to_string(), "add".to_string()];
    if transport != "stdio" {
        argv.push("-t".into());
        argv.push(transport.clone());
    }
    argv.push("-s".into());
    argv.push(scope);
    for item in spec.env {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if !item.contains('=') {
            bail!("env must be KEY=value");
        }
        argv.push("-e".into());
        argv.push(item.to_string());
    }
    for item in spec.headers {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        argv.push("-H".into());
        argv.push(item.to_string());
    }
    argv.push(spec.name.to_string());
    if transport == "stdio" {
        argv.push("--".into());
        argv.push(spec.command_or_url.to_string());
        argv.extend(spec.args.iter().cloned());
    } else {
        argv.push(spec.command_or_url.to_string());
    }
    run_text(bin, cwd, &argv).await
}

/// # Errors
/// Returns an error if the name or scope is invalid, or grok `mcp remove` fails.
pub async fn remove(bin: &Path, cwd: &Path, name: &str, scope: Option<&str>) -> Result<String> {
    check_name(name)?;
    let mut argv = vec!["mcp".to_string(), "remove".to_string(), name.to_string()];
    if let Some(scope) = scope {
        let scope = normalize_scope(scope)?;
        argv.push("-s".into());
        argv.push(scope);
    }
    run_text(bin, cwd, &argv).await
}

/// # Errors
/// Returns an error if the name is invalid or grok `mcp enable` fails.
pub async fn enable(bin: &Path, cwd: &Path, name: &str) -> Result<String> {
    check_name(name)?;
    run_text(bin, cwd, &["mcp".into(), "enable".into(), name.into()]).await
}

/// # Errors
/// Returns an error if the name is invalid or grok `mcp disable` fails.
pub async fn disable(bin: &Path, cwd: &Path, name: &str) -> Result<String> {
    check_name(name)?;
    run_text(bin, cwd, &["mcp".into(), "disable".into(), name.into()]).await
}

fn check_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("name required");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("name may only contain letters, numbers, hyphens, and underscores");
    }
    Ok(())
}

fn normalize_transport(raw: &str) -> Result<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "stdio" => Ok("stdio".into()),
        "http" => Ok("http".into()),
        "sse" => Ok("sse".into()),
        other => bail!("unknown transport {other}"),
    }
}

fn normalize_scope(raw: &str) -> Result<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "user" => Ok("user".into()),
        "project" => Ok("project".into()),
        other => bail!("unknown scope {other}"),
    }
}

async fn run_json(bin: &Path, cwd: &Path, args: &[&str]) -> Result<Value> {
    let text = run_raw(bin, cwd, args).await?;
    if text.is_empty() {
        return Ok(json!([]));
    }
    match serde_json::from_str(&text) {
        Ok(v) => Ok(v),
        Err(_) => Ok(json!({ "text": text })),
    }
}

async fn run_text(bin: &Path, cwd: &Path, args: &[String]) -> Result<String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    run_raw(bin, cwd, &args).await
}

async fn run_raw(bin: &Path, cwd: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let out = timeout(MCP_TIMEOUT, cmd.output())
        .await
        .context("grok mcp timed out")?
        .with_context(|| format!("spawn grok mcp {}", args.join(" ")))?;
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
        format!("grok mcp exited {}", out.status)
    };
    bail!("{msg}")
}

fn merge(list: &Value, doctor: &Value) -> Value {
    let mut by_name: serde_json::Map<String, Value> = serde_json::Map::new();
    for item in iter_servers(list) {
        if let Some(name) = server_name(&item) {
            by_name.insert(name, item);
        }
    }
    if let Some(servers) = doctor.get("servers").and_then(Value::as_array) {
        for item in servers {
            let Some(name) = server_name(item) else {
                continue;
            };
            let entry = by_name
                .entry(name.clone())
                .or_insert_with(|| json!({ "name": name }));
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("doctor".into(), item.clone());
                for key in ["source", "transport", "target", "healthy"] {
                    if obj.get(key).is_none()
                        && let Some(v) = item.get(key)
                    {
                        obj.insert(key.to_string(), v.clone());
                    }
                }
            }
        }
    }
    let mut servers: Vec<Value> = by_name.into_values().collect();
    for server in &mut servers {
        hoist_tools(server);
    }
    servers.sort_by(|a, b| {
        server_name(a)
            .unwrap_or_default()
            .to_lowercase()
            .cmp(&server_name(b).unwrap_or_default().to_lowercase())
    });
    json!({
        "servers": servers,
        "sources": doctor.get("sources").cloned().unwrap_or(json!([])),
        "healthy_count": doctor.get("healthy_count").cloned().unwrap_or(json!(0)),
        "failing_count": doctor.get("failing_count").cloned().unwrap_or(json!(0)),
    })
}

fn iter_servers(list: &Value) -> Vec<Value> {
    if let Some(arr) = list.as_array() {
        return arr.iter().cloned().map(coerce_item).collect();
    }
    if let Some(arr) = list.get("servers").and_then(Value::as_array) {
        return arr.iter().cloned().map(coerce_item).collect();
    }
    Vec::new()
}

fn coerce_item(item: Value) -> Value {
    if let Some(name) = item.as_str() {
        return json!({ "name": name, "enabled": true });
    }
    item
}

fn server_name(item: &Value) -> Option<String> {
    item.get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| item.as_str().map(ToOwned::to_owned))
}

fn hoist_tools(server: &mut Value) {
    if server
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty())
    {
        return;
    }
    let tools = server.get("doctor").and_then(|d| {
        d.get("tools")
            .cloned()
            .or_else(|| d.get("available_tools").cloned())
            .or_else(|| d.get("toolList").cloned())
    });
    if let Some(tools) = tools
        && let Some(obj) = server.as_object_mut()
    {
        obj.insert("tools".into(), tools);
    }
}
