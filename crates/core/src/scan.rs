use crate::types::{ProjectRow, SessionMeta, SessionRow, SubagentMeta, SummaryFile};
use anyhow::Result;
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct SessionIndex {
    pub sessions: HashMap<String, SessionMeta>,
}

impl SessionIndex {
    #[must_use]
    pub fn projects(&self) -> Vec<ProjectRow> {
        let mut by_cwd: HashMap<String, (usize, i64, String)> = HashMap::new();
        for s in self.sessions.values() {
            let entry = by_cwd.entry(s.cwd.clone()).or_insert((0, 0, String::new()));
            entry.0 += 1;
            if s.updated_sort >= entry.1 {
                entry.1 = s.updated_sort;
                entry.2.clone_from(&s.updated_at);
            }
        }
        let mut rows: Vec<ProjectRow> = by_cwd
            .into_iter()
            .map(|(cwd, (sessions, _, updated_at))| ProjectRow {
                cwd,
                sessions,
                updated_at,
            })
            .collect();
        rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.cwd.cmp(&b.cwd)));
        rows
    }

    #[must_use]
    pub fn list(
        &self,
        cwd: Option<&str>,
        q: Option<&str>,
        empty: bool,
        fts_ids: Option<&[String]>,
    ) -> Vec<SessionRow> {
        let q_lower = q.map(str::to_lowercase).filter(|s| !s.is_empty());
        let mut rows: Vec<&SessionMeta> = self
            .sessions
            .values()
            .filter(|s| cwd.is_none_or(|c| s.cwd == c))
            .filter(|s| empty || !s.empty)
            .filter(|s| {
                let Some(q) = q_lower.as_deref() else {
                    return true;
                };
                if s.title.to_lowercase().contains(q)
                    || s.cwd.to_lowercase().contains(q)
                    || s.id.to_lowercase().contains(q)
                {
                    return true;
                }
                fts_ids.is_some_and(|ids| ids.iter().any(|id| id == &s.id))
            })
            .collect();
        rows.sort_by(|a, b| b.updated_sort.cmp(&a.updated_sort).then(a.id.cmp(&b.id)));
        rows.into_iter().map(SessionMeta::to_row).collect()
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&SessionMeta> {
        self.sessions.get(id)
    }
}

/// # Errors
/// Returns an error if a sessions directory entry cannot be read.
pub fn scan(grok_home: &Path) -> Result<SessionIndex> {
    let root = grok_home.join("sessions");
    let mut index = SessionIndex::default();
    if !root.is_dir() {
        return Ok(index);
    }
    let mut child_to_parent: HashMap<String, String> = HashMap::new();
    for project_ent in fs::read_dir(&root)? {
        let project_ent = project_ent?;
        let project_path = project_ent.path();
        if !project_path.is_dir() {
            continue;
        }
        let cwd = project_cwd(&project_path);
        for sess_ent in fs::read_dir(&project_path)? {
            let sess_ent = sess_ent?;
            let sess_path = sess_ent.path();
            if !sess_path.is_dir() {
                continue;
            }
            let name = sess_ent.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if Uuid::parse_str(name).is_err() {
                continue;
            }
            collect_subagent_parents(&sess_path, &mut child_to_parent);
            if let Some(meta) = load_session_meta(&sess_path, name, &cwd) {
                index.sessions.insert(meta.id.clone(), meta);
            }
        }
    }
    for (child, parent) in child_to_parent {
        if let Some(meta) = index.sessions.get_mut(&child)
            && meta.parent_id.is_none()
        {
            meta.parent_id = Some(parent);
        }
    }
    Ok(index)
}

fn project_cwd(project_path: &Path) -> String {
    let cwd_file = project_path.join(".cwd");
    if let Ok(s) = fs::read_to_string(&cwd_file) {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let name = project_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    percent_decode_str(name).decode_utf8_lossy().into_owned()
}

fn collect_subagent_parents(sess_path: &Path, map: &mut HashMap<String, String>) {
    let sub = sess_path.join("subagents");
    let Ok(entries) = fs::read_dir(&sub) else {
        return;
    };
    for ent in entries.flatten() {
        let meta_path = ent.path().join("meta.json");
        let Ok(raw) = fs::read_to_string(&meta_path) else {
            continue;
        };
        let Ok(meta) = serde_json::from_str::<SubagentMeta>(&raw) else {
            continue;
        };
        if let (Some(child), Some(parent)) = (meta.child_session_id, meta.parent_session_id) {
            map.insert(child, parent);
        }
    }
}

fn load_session_meta(sess_path: &Path, dir_id: &str, fallback_cwd: &str) -> Option<SessionMeta> {
    let summary_path = sess_path.join("summary.json");
    let raw = fs::read_to_string(&summary_path).ok()?;
    let summary: SummaryFile = serde_json::from_str(&raw).ok()?;
    let updates = sess_path.join("updates.jsonl");
    let empty = !updates.is_file() || summary.num_messages == 0;
    let title = summary
        .generated_title
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(summary.session_summary.as_deref().filter(|s| !s.is_empty()))
        .unwrap_or(dir_id)
        .to_string();
    let cwd = summary
        .info
        .cwd
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_cwd)
        .to_string();
    let created_at = summary.created_at.clone().unwrap_or_default();
    let updated_at = summary
        .updated_at
        .clone()
        .unwrap_or_else(|| created_at.clone());
    let updated_sort = parse_sort_ts(&updated_at);
    let parent_id = summary.parent_session_id.filter(|s| !s.is_empty());
    Some(SessionMeta {
        id: if summary.info.id.is_empty() {
            dir_id.to_string()
        } else {
            summary.info.id
        },
        cwd,
        title,
        created_at,
        updated_at,
        updated_sort,
        model: summary.current_model_id.unwrap_or_default(),
        agent_name: summary.agent_name.unwrap_or_default(),
        num_messages: summary.num_messages,
        parent_id,
        empty,
        dir: sess_path.to_path_buf(),
    })
}

fn parse_sort_ts(s: &str) -> i64 {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp();
    }
    if let Some((head, _rest)) = s.split_once('.') {
        let rebuilt = format!("{head}Z");
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&rebuilt) {
            return dt.timestamp();
        }
    }
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(0)
}
