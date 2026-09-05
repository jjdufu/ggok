use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const TITLE_MAX: usize = 200;

#[derive(Debug, Default, Serialize, Deserialize)]
struct PinsFile {
    #[serde(default)]
    ids: Vec<String>,
}

fn valid_title(raw: &str) -> Result<String> {
    let title = raw.trim();
    if title.is_empty() {
        bail!("title is empty");
    }
    if title.chars().count() > TITLE_MAX {
        bail!("title too long");
    }
    if title.chars().any(char::is_control) {
        bail!("title has control characters");
    }
    Ok(title.to_string())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("rename {}", path.display()))?;
    Ok(())
}

#[must_use]
pub fn load_pins(path: &Path) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let parsed: PinsFile = serde_json::from_str(&raw).unwrap_or_default();
    parsed
        .ids
        .into_iter()
        .filter(|id| Uuid::parse_str(id).is_ok())
        .collect()
}

#[must_use]
pub fn is_pinned(path: &Path, id: &str) -> bool {
    load_pins(path).iter().any(|x| x == id)
}

/// # Errors
/// Returns an error if `id` is not a UUID or the pins file cannot be written.
pub fn set_pinned(path: &Path, id: &str, pinned: bool) -> Result<Vec<String>> {
    if Uuid::parse_str(id).is_err() {
        bail!("invalid session id");
    }
    let mut ids = load_pins(path);
    ids.retain(|x| x != id);
    if pinned {
        ids.insert(0, id.to_string());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let body = PinsFile { ids: ids.clone() };
    let bytes = serde_json::to_vec_pretty(&body).context("serialize pins")?;
    atomic_write(path, &bytes)?;
    Ok(ids)
}

/// # Errors
/// Returns an error if the title is invalid or `summary.json` cannot be read or written.
pub fn rename_summary(dir: &Path, id: &str, cwd: &str, title: &str) -> Result<String> {
    let title = valid_title(title)?;
    let path = dir.join("summary.json");
    let mut value = if path.is_file() {
        let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str::<Value>(&raw).with_context(|| format!("parse {}", path.display()))?
    } else {
        json!({
            "info": { "id": id, "cwd": cwd }
        })
    };
    let obj = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("summary.json is not an object"))?;
    obj.insert("generated_title".into(), json!(title));
    obj.insert("session_summary".into(), json!(title));
    obj.insert("title_is_manual".into(), json!(true));
    if !obj.contains_key("info") {
        obj.insert("info".into(), json!({ "id": id, "cwd": cwd }));
    }
    let bytes = serde_json::to_vec_pretty(&value).context("serialize summary")?;
    atomic_write(&path, &bytes)?;
    Ok(title)
}

#[must_use]
pub fn pins_path_from_agent_pid(agent_pid_file: &Path) -> PathBuf {
    agent_pid_file
        .parent()
        .map_or_else(|| PathBuf::from("pins.json"), |p| p.join("pins.json"))
}
