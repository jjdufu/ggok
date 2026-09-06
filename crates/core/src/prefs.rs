use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Last model and reasoning effort chosen in the web UI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastModel {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub effort: String,
}

/// Path of ggok's last-model file under a config directory.
#[must_use]
pub fn last_model_path(config_dir: &Path) -> PathBuf {
    config_dir.join("last_model.json")
}

/// Load last model/effort. Missing or invalid files yield an empty value.
#[must_use]
pub fn load_last_model(path: &Path) -> LastModel {
    let Ok(raw) = fs::read_to_string(path) else {
        return LastModel::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// # Errors
/// Returns an error if the parent directory cannot be created or the file cannot be written.
pub fn save_last_model(path: &Path, last: &LastModel) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let json = serde_json::to_vec_pretty(last).context("serialize last model")?;
    fs::write(path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Prefer an explicit model/effort, otherwise the stored last-used pair.
#[must_use]
pub fn resolve_choice(
    model: Option<&str>,
    effort: Option<&str>,
    last: &LastModel,
) -> (Option<String>, Option<String>) {
    let model = nonempty(model);
    let effort = nonempty(effort);
    if model.is_some() || effort.is_some() {
        return (model, effort);
    }
    (
        nonempty(Some(last.model.as_str())),
        nonempty(Some(last.effort.as_str())),
    )
}

/// Persist last-used model for ggok and grok new sessions.
///
/// Writes ggok `last_model.json` when the config dir can be resolved, and merges
/// `[models] default` / `default_reasoning_effort` into `grok_home/config.toml`.
pub fn remember_model_choice(grok_home: &Path, model: &str, effort: &str) {
    if model.is_empty() {
        return;
    }
    if let Ok(dir) = crate::config::config_dir() {
        let last = LastModel {
            model: model.to_string(),
            effort: effort.to_string(),
        };
        let _ = save_last_model(&last_model_path(&dir), &last);
    }
    let _ = merge_grok_model_defaults(&grok_home.join("config.toml"), model, effort);
}

/// Merge grok `[models]` default keys without rewriting other tables.
///
/// # Errors
/// Returns an error if the file cannot be written.
pub fn merge_grok_model_defaults(path: &Path, model: &str, effort: &str) -> Result<()> {
    if model.is_empty() {
        return Ok(());
    }
    let src = fs::read_to_string(path).unwrap_or_default();
    let next = merge_models_section(&src, model, effort);
    if next == src {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(path, next).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Rewrite only `[models] default` and `default_reasoning_effort` in a TOML document.
#[must_use]
pub fn merge_models_section(src: &str, model: &str, effort: &str) -> String {
    if model.is_empty() {
        return src.to_string();
    }
    let mut out = String::with_capacity(src.len() + 96);
    let mut in_models = false;
    let mut seen_models = false;
    let mut wrote_default = false;
    let mut wrote_effort = false;
    let mut pending_nl = false;

    for line in src.lines() {
        if pending_nl {
            out.push('\n');
        }
        pending_nl = true;
        let trimmed = line.trim();
        if is_table_header(trimmed) {
            if in_models {
                write_missing_keys(&mut out, model, effort, wrote_default, wrote_effort);
                wrote_default = true;
                wrote_effort = true;
            }
            in_models = trimmed == "[models]";
            if in_models {
                seen_models = true;
                wrote_default = false;
                wrote_effort = false;
            }
            out.push_str(line);
            continue;
        }
        if in_models {
            match line_key(trimmed) {
                Some("default") => {
                    out.push_str(&key_line("default", model));
                    wrote_default = true;
                    continue;
                }
                Some("default_reasoning_effort") => {
                    if effort.is_empty() {
                        out.push_str(line);
                    } else {
                        out.push_str(&key_line("default_reasoning_effort", effort));
                        wrote_effort = true;
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push_str(line);
    }
    if in_models {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        write_missing_keys(&mut out, model, effort, wrote_default, wrote_effort);
    } else if !seen_models {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("[models]\n");
        out.push_str(&key_line("default", model));
        out.push('\n');
        if !effort.is_empty() {
            out.push_str(&key_line("default_reasoning_effort", effort));
            out.push('\n');
        }
    }
    if src.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn nonempty(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn is_table_header(trimmed: &str) -> bool {
    trimmed.starts_with('[') && trimmed.ends_with(']')
}

fn line_key(trimmed: &str) -> Option<&str> {
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    trimmed.split_once('=').map(|(k, _)| k.trim())
}

fn key_line(key: &str, value: &str) -> String {
    format!("{key} = \"{}\"", escape_toml_string(value))
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn write_missing_keys(
    out: &mut String,
    model: &str,
    effort: &str,
    wrote_default: bool,
    wrote_effort: bool,
) {
    if !wrote_default {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&key_line("default", model));
        out.push('\n');
    }
    if !effort.is_empty() && !wrote_effort {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&key_line("default_reasoning_effort", effort));
        out.push('\n');
    }
}
