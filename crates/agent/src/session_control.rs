use super::Agent;
use anyhow::{Result, bail};
use ggok_core::types::{Block, PromptFile, QueueItem, TodoItem};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;

/// Prefix for HTTP 501 when grok does not expose the ACP method.
pub const ACP_UNAVAILABLE: &str = "acp_method_unavailable";

const REWIND_POINTS: &str = "x.ai/rewind/points";
const REWIND_EXECUTE: &str = "x.ai/rewind/execute";
const SESSION_FORK: &str = "x.ai/session/fork";
const COMPACT: &str = "x.ai/compact_conversation";
const BTW: &str = "x.ai/btw";
const TASK_KILL: &str = "x.ai/task/kill";
const SUBAGENT_LIST: &str = "x.ai/subagent/list";
const TOGGLE_PLAN: &str = "x.ai/toggle_plan_mode";
const WORKFLOWS_LIST: &str = "x.ai/workflows/list";
const HOOKS_ACTION: &str = "x.ai/hooks/action";

/// A rewind target, normalized from grok's `x.ai/rewind/points` payload.
#[derive(Debug, Clone, Serialize)]
pub struct RewindPoint {
    pub prompt_index: u64,
    pub preview: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Result of `x.ai/session/fork`.
#[derive(Debug, Clone, Serialize)]
pub struct ForkedSession {
    pub id: String,
    pub cwd: String,
    pub parent_id: String,
}

/// One row from `x.ai/subagent/list`.
#[derive(Debug, Clone, Serialize)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// `plan.md` plus ACP todos for the Plan preview.
#[derive(Debug, Clone, Serialize)]
pub struct SessionPlan {
    pub markdown: String,
    pub todos: Vec<TodoItem>,
}

impl Agent {
    /// # Errors
    /// Returns an error if the session is not attached or grok is down.
    pub async fn rewind_points(&self, id: &str, cwd: &str) -> Result<Vec<RewindPoint>> {
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        match self
            .call(REWIND_POINTS, json!({ "sessionId": id }))
            .await
        {
            Ok(v) => {
                let mut points = normalize_rewind_points(&v);
                if points.is_empty() {
                    points = self.points_from_blocks(id).await;
                }
                Ok(points)
            }
            Err(e) => {
                if is_method_missing(&e) {
                    Ok(self.points_from_blocks(id).await)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// # Errors
    /// Returns an error if occupancy blocks the op or ACP rewind fails.
    pub async fn rewind_execute(&self, id: &str, cwd: &str, prompt_index: u64) -> Result<u64> {
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        self.cancel_if_running(id).await?;
        tracing::info!(session_id = %id, prompt_index, "rewind execute");
        let params = json!({
            "sessionId": id,
            "promptIndex": prompt_index,
            "prompt_index": prompt_index
        });
        self.call(REWIND_EXECUTE, params).await?;
        self.emit(id, "rewind", &json!({ "prompt_index": prompt_index }));
        self.emit(id, "resync", &json!({}));
        Ok(prompt_index)
    }

    /// # Errors
    /// Returns an error if rewind or the follow-up prompt fails.
    pub async fn retry(
        &self,
        id: &str,
        cwd: &str,
        prompt_index: u64,
        text: Option<String>,
        files: Vec<PromptFile>,
    ) -> Result<super::PromptOutcome> {
        self.require_attached(id).await?;
        let mut body = text.unwrap_or_default();
        if body.trim().is_empty() {
            body = self
                .preview_at(id, cwd, prompt_index)
                .await
                .unwrap_or_default();
        }
        if body.trim().is_empty() {
            bail!("retry text is empty");
        }
        self.rewind_execute(id, cwd, prompt_index).await?;
        self.wait_not_running(id).await;
        let item = QueueItem {
            id: Uuid::new_v4().to_string(),
            text: body,
            files,
        };
        self.start_prompt(id, item).await?;
        Ok(super::PromptOutcome {
            queued: false,
            queue: Vec::new(),
        })
    }

    /// # Errors
    /// Returns an error if occupancy blocks the op, worktree is requested, or ACP fork fails.
    pub async fn fork_session(
        &self,
        id: &str,
        cwd: &str,
        directive: Option<&str>,
        worktree: bool,
    ) -> Result<ForkedSession> {
        if worktree {
            bail!("worktree fork is not implemented");
        }
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        self.cancel_if_running(id).await?;
        let mut params = json!({ "sessionId": id, "worktree": false });
        if let Some(d) = directive.map(str::trim).filter(|s| !s.is_empty())
            && let Some(obj) = params.as_object_mut()
        {
            obj.insert("directive".into(), json!(d));
        }
        tracing::info!(session_id = %id, "session fork");
        let result = match self.call(SESSION_FORK, params).await {
            Ok(v) => v,
            Err(e) if is_method_missing(&e) => {
                bail!("{ACP_UNAVAILABLE}: {SESSION_FORK}");
            }
            Err(e) => return Err(e),
        };
        let new_id = json_str(
            &result,
            &[
                "sessionId",
                "session_id",
                "id",
                "newSessionId",
                "new_session_id",
            ],
        );
        if new_id.is_empty() {
            bail!("{ACP_UNAVAILABLE}: {SESSION_FORK}");
        }
        let new_cwd = json_str(&result, &["cwd", "workingDirectory", "working_directory"]);
        let new_cwd = if new_cwd.is_empty() {
            cwd.to_string()
        } else {
            new_cwd
        };
        Ok(ForkedSession {
            id: new_id,
            cwd: new_cwd,
            parent_id: id.to_string(),
        })
    }

    /// # Errors
    /// Returns an error if occupancy blocks the op or ACP compact fails.
    pub async fn compact_session(&self, id: &str, cwd: &str, context: Option<&str>) -> Result<()> {
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        self.cancel_if_running(id).await?;
        let mut params = json!({ "sessionId": id });
        if let Some(ctx) = context.map(str::trim).filter(|s| !s.is_empty())
            && let Some(obj) = params.as_object_mut()
        {
            obj.insert("context".into(), json!(ctx));
        }
        tracing::info!(session_id = %id, "compact conversation");
        self.emit(
            id,
            "compact",
            &json!({ "phase": "start", "message": "" }),
        );
        match self.call(COMPACT, params).await {
            Ok(_) => {
                self.emit(id, "compact", &json!({ "phase": "done", "message": "" }));
                self.emit(id, "resync", &json!({}));
                Ok(())
            }
            Err(e) if is_method_missing(&e) => {
                self.emit(
                    id,
                    "compact",
                    &json!({ "phase": "error", "message": e.to_string() }),
                );
                bail!("{ACP_UNAVAILABLE}: {COMPACT}");
            }
            Err(e) => {
                self.emit(
                    id,
                    "compact",
                    &json!({ "phase": "error", "message": e.to_string() }),
                );
                Err(e)
            }
        }
    }

    /// # Errors
    /// Returns an error if the mode is invalid or the session is not attached.
    pub async fn set_session_mode(&self, id: &str, cwd: &str, mode: &str) -> Result<String> {
        let mode = normalize_mode(mode)?;
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        self.cancel_if_running(id).await?;
        tracing::info!(session_id = %id, mode, "set session mode");
        let applied = if mode == "plan" {
            self.set_plan_mode(id, true).await
        } else {
            let _ = self.set_plan_mode(id, false).await;
            self.set_permission_mode(id, mode).await || mode == "ask"
        };
        if !applied {
            tracing::warn!(session_id = %id, mode, "mode apply via acp failed");
        }
        {
            let mut g = self.inner.lock().await;
            if let Some(sess) = g.sessions.get_mut(id) {
                sess.mode = mode.to_string();
            }
        }
        self.emit(id, "mode", &json!({ "mode": mode }));
        Ok(mode.to_string())
    }

    /// # Errors
    /// Returns an error if occupancy blocks the op or ACP btw fails.
    pub async fn btw(&self, id: &str, cwd: &str, text: &str) -> Result<()> {
        let text = text.trim();
        if text.is_empty() {
            bail!("btw text is empty");
        }
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        match self
            .call(BTW, json!({ "sessionId": id, "text": text, "prompt": text }))
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if is_method_missing(&e) => bail!("{ACP_UNAVAILABLE}: {BTW}"),
            Err(e) => Err(e),
        }
    }

    /// # Errors
    /// Returns an error if occupancy blocks the op or the list method is missing.
    pub async fn list_tasks(&self, id: &str, cwd: &str) -> Result<Vec<TaskRow>> {
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        match self.call(SUBAGENT_LIST, json!({ "sessionId": id })).await {
            Ok(v) => Ok(normalize_tasks(&v)),
            Err(e) if is_method_missing(&e) => bail!("{ACP_UNAVAILABLE}: {SUBAGENT_LIST}"),
            Err(e) => Err(e),
        }
    }

    /// # Errors
    /// Returns an error if occupancy blocks the op or ACP kill fails.
    pub async fn kill_task(&self, id: &str, cwd: &str, task_id: &str) -> Result<()> {
        if task_id.trim().is_empty() {
            bail!("task id is empty");
        }
        self.require_attached(id).await?;
        self.ensure().await?;
        self.session_load_inner(id, cwd).await?;
        match self
            .call(
                TASK_KILL,
                json!({ "sessionId": id, "taskId": task_id, "task_id": task_id, "id": task_id }),
            )
            .await
        {
            Ok(_) => {
                self.emit(id, "tasks", &json!({ "killed": task_id }));
                Ok(())
            }
            Err(e) if is_method_missing(&e) => bail!("{ACP_UNAVAILABLE}: {TASK_KILL}"),
            Err(e) => Err(e),
        }
    }

    /// # Errors
    /// Returns an error if the session directory cannot be resolved.
    pub async fn session_plan(&self, id: &str, cwd: &str) -> Result<SessionPlan> {
        let todos = {
            let live = self.live_todos(id).await;
            if live.is_empty() {
                None
            } else {
                Some(live)
            }
        };
        let dir = session_dir(&self.grok_home, cwd, id);
        let markdown = std::fs::read_to_string(dir.join("plan.md")).unwrap_or_default();
        let todos = todos.unwrap_or_else(|| {
            // Disk parse may have todos from plan updates.
            ggok_core::parse::parse_updates_file(&dir.join("updates.jsonl"))
                .map(|p| p.todos)
                .unwrap_or_default()
        });
        Ok(SessionPlan { markdown, todos })
    }

    /// # Errors
    /// Returns an error if grok is down or the list method is missing.
    pub async fn list_workflows(&self, cwd: &Path) -> Result<Value> {
        self.ensure().await?;
        match self
            .call(WORKFLOWS_LIST, json!({ "cwd": cwd.to_string_lossy() }))
            .await
        {
            Ok(v) => Ok(v),
            Err(e) if is_method_missing(&e) => Ok(scan_workflow_dirs(cwd)),
            Err(e) => Err(e),
        }
    }

    /// # Errors
    /// Returns an error if grok is down or the hooks method is missing.
    pub async fn list_hooks(&self, cwd: &Path) -> Result<Value> {
        self.ensure().await?;
        match self
            .call(
                HOOKS_ACTION,
                json!({ "cwd": cwd.to_string_lossy(), "action": "list" }),
            )
            .await
        {
            Ok(v) => Ok(v),
            Err(e) if is_method_missing(&e) => {
                bail!("{ACP_UNAVAILABLE}: {HOOKS_ACTION}")
            }
            Err(e) => Err(e),
        }
    }

    async fn cancel_if_running(&self, id: &str) -> Result<()> {
        let running = {
            let g = self.inner.lock().await;
            g.sessions.get(id).is_some_and(|s| s.running)
        };
        if running {
            self.cancel(id).await?;
            self.wait_not_running(id).await;
        }
        Ok(())
    }

    async fn wait_not_running(&self, id: &str) {
        for _ in 0..80 {
            let running = {
                let g = self.inner.lock().await;
                g.sessions.get(id).is_some_and(|s| s.running)
            };
            if !running {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    async fn points_from_blocks(&self, id: &str) -> Vec<RewindPoint> {
        let blocks = {
            let g = self.inner.lock().await;
            g.sessions
                .get(id)
                .map(|s| s.parser.snapshot_blocks())
                .unwrap_or_default()
        };
        user_points_from_blocks(&blocks)
    }

    async fn preview_at(&self, id: &str, cwd: &str, prompt_index: u64) -> Option<String> {
        if let Ok(points) = self.rewind_points(id, cwd).await
            && let Some(p) = points.iter().find(|p| p.prompt_index == prompt_index)
            && !p.preview.is_empty()
        {
            return Some(p.preview.clone());
        }
        let blocks = {
            let g = self.inner.lock().await;
            g.sessions
                .get(id)
                .map(|s| s.parser.snapshot_blocks())
                .unwrap_or_default()
        };
        user_points_from_blocks(&blocks)
            .into_iter()
            .find(|p| p.prompt_index == prompt_index)
            .map(|p| p.preview)
    }

    pub(crate) async fn set_plan_mode(&self, id: &str, enabled: bool) -> bool {
        self.call(
            TOGGLE_PLAN,
            json!({ "sessionId": id, "enabled": enabled, "value": enabled }),
        )
        .await
        .is_ok()
    }

    pub(crate) async fn set_permission_mode(&self, id: &str, mode: &str) -> bool {
        let yolo = ["always-approve", "yolo"];
        let one = [mode];
        let ids: &[&str] = if mode == "always-approve" {
            &yolo
        } else {
            &one
        };
        for config_id in ids {
            if self
                .call(
                    "session/set_config_option",
                    json!({ "sessionId": id, "configId": *config_id, "value": true }),
                )
                .await
                .is_ok()
            {
                return true;
            }
        }
        self.call(
            "x.ai/set_permission_mode",
            json!({ "sessionId": id, "mode": mode }),
        )
        .await
        .is_ok()
    }
}

pub(crate) fn normalize_mode(mode: &str) -> Result<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "ask" => Ok("ask"),
        "auto" => Ok("auto"),
        "always-approve" | "always_approve" | "yolo" => Ok("always-approve"),
        "plan" => Ok("plan"),
        _ => bail!("invalid mode"),
    }
}

fn is_method_missing(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("method not found")
        || (msg.contains("not found") && msg.contains("method"))
        || msg.contains("-32601")
}

fn json_str(v: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = v.get(*key).and_then(Value::as_str).map(str::trim)
            && !s.is_empty()
        {
            return s.to_string();
        }
        if let Some(s) = v.pointer(&format!("/{key}")).and_then(Value::as_str).map(str::trim)
            && !s.is_empty()
        {
            return s.to_string();
        }
    }
    String::new()
}

fn json_u64(v: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        let n = v.get(*key).and_then(value_u64);
        if n.is_some() {
            return n;
        }
    }
    None
}

fn value_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
}

fn normalize_rewind_points(v: &Value) -> Vec<RewindPoint> {
    let arr = v
        .get("points")
        .or_else(|| v.get("rewindPoints"))
        .or_else(|| v.get("items"))
        .or_else(|| v.as_array().map(|_| v))
        .and_then(Value::as_array);
    let Some(arr) = arr else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let prompt_index = json_u64(row, &["prompt_index", "promptIndex", "index"])?;
            let preview = json_str(row, &["preview", "prompt_preview", "text", "content"]);
            let created_at = json_str(row, &["created_at", "createdAt"]);
            let disabled = row
                .get("disabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let reason = row
                .get("reason")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            Some(RewindPoint {
                prompt_index,
                preview,
                created_at,
                disabled,
                reason,
            })
        })
        .collect()
}

fn user_points_from_blocks(blocks: &[Block]) -> Vec<RewindPoint> {
    let mut out = Vec::new();
    for block in blocks {
        if let Block::User { text, .. } = block {
            let preview: String = text.chars().take(80).collect();
            out.push(RewindPoint {
                prompt_index: u64::try_from(out.len()).unwrap_or(0),
                preview,
                created_at: String::new(),
                disabled: false,
                reason: None,
            });
        }
    }
    out
}

fn normalize_tasks(v: &Value) -> Vec<TaskRow> {
    let arr = v
        .get("tasks")
        .or_else(|| v.get("subagents"))
        .or_else(|| v.get("items"))
        .or_else(|| v.as_array().map(|_| v))
        .and_then(Value::as_array);
    let Some(arr) = arr else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let id = json_str(row, &["id", "taskId", "task_id", "sessionId", "session_id"]);
            if id.is_empty() {
                return None;
            }
            let title = json_str(row, &["title", "name", "description"]);
            let status = json_str(row, &["status", "state"]);
            Some(TaskRow {
                id,
                title,
                status,
            })
        })
        .collect()
}

fn session_dir(grok_home: &Path, cwd: &str, id: &str) -> std::path::PathBuf {
    grok_home
        .join("sessions")
        .join(
            percent_encoding::utf8_percent_encode(cwd, percent_encoding::NON_ALPHANUMERIC)
                .to_string(),
        )
        .join(id)
}

fn scan_workflow_dirs(cwd: &Path) -> Value {
    let mut names = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        cwd.join(".grok/workflows"),
        Path::new(&home).join(".grok/workflows"),
    ];
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in entries.flatten() {
            let path = ent.path();
            if path.extension().and_then(|s| s.to_str()) != Some("rhai") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(json!({ "name": stem, "path": path.display().to_string() }));
            }
        }
    }
    json!({ "workflows": names })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_rewind_points_snake_and_camel() {
        let v = json!({
            "points": [
                {"prompt_index": 1, "preview": "hello", "created_at": "t"},
                {"promptIndex": 2, "text": "world", "disabled": true, "reason": "compacted"}
            ]
        });
        let pts = normalize_rewind_points(&v);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].prompt_index, 1);
        assert_eq!(pts[1].prompt_index, 2);
        assert!(pts[1].disabled);
        assert_eq!(pts[1].reason.as_deref(), Some("compacted"));
    }

    #[test]
    fn mode_aliases() {
        assert_eq!(normalize_mode("YOLO").unwrap(), "always-approve");
        assert_eq!(normalize_mode("plan").unwrap(), "plan");
        assert!(normalize_mode("nope").is_err());
    }
}
