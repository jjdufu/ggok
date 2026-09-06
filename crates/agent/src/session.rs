use super::rpc::write_stdin;
use super::{
    Agent, NewSession, PermOpt, PromptOutcome, live_entry, now_ms, reset_parser_keep_usage,
};
use anyhow::{Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ggok_core::occupy::{SESSION_BUSY, Source};
use ggok_core::parse::Ingest;
use ggok_core::types::{Block, EffortInfo, ModelInfo, PromptFile, QueueItem, SlashCommand};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use uuid::Uuid;

impl Agent {
    /// # Errors
    /// Returns an error if the grok process is down or `session/new` fails.
    pub async fn session_new(
        &self,
        cwd: &Path,
        model: Option<&str>,
        effort: Option<&str>,
    ) -> Result<NewSession> {
        self.ensure().await?;
        let params = json!({
            "cwd": cwd.to_string_lossy(),
            "mcpServers": self.ask_mcp_servers(),
            "_meta": crate::question::acp_session_meta(&self.permission_mode)
        });
        let result = self.call("session/new", params).await?;
        let id = result
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("session/new missing sessionId"))?
            .to_string();
        self.yield_web_active(&id).await;
        self.apply_session_result(&id, cwd.to_string_lossy().as_ref(), &result)
            .await;
        if model.as_ref().is_some_and(|s| !s.is_empty())
            || effort.as_ref().is_some_and(|s| !s.is_empty())
        {
            let chosen = {
                let g = self.inner.lock().await;
                model.filter(|s| !s.is_empty()).map_or_else(
                    || {
                        g.sessions
                            .get(&id)
                            .map_or_else(|| g.current_model.clone(), |s| s.model.clone())
                    },
                    ToOwned::to_owned,
                )
            };
            if !chosen.is_empty() {
                let _ = self.set_model(&id, &chosen, effort).await;
            }
        }
        let g = self.inner.lock().await;
        let model = g
            .sessions
            .get(&id)
            .map_or_else(|| g.current_model.clone(), |s| s.model.clone());
        Ok(NewSession {
            id,
            cwd: cwd.to_string_lossy().into_owned(),
            model,
        })
    }

    /// # Errors
    /// Returns an error if the session is occupied, the grok process is down, or `session/load` fails.
    pub async fn session_load(&self, id: &str, cwd: &str) -> Result<()> {
        {
            let g = self.inner.lock().await;
            if g.sessions.get(id).is_some_and(|s| s.loaded) {
                return Ok(());
            }
        }
        let occ = self.occupancy_of(id, Some(cwd)).await;
        if occ.source.is_spectator() {
            bail!(SESSION_BUSY);
        }
        self.ensure().await?;
        self.session_load_inner(id, cwd).await
    }

    pub(crate) async fn session_load_inner(&self, id: &str, cwd: &str) -> Result<()> {
        {
            let g = self.inner.lock().await;
            if g.sessions.get(id).is_some_and(|s| s.loaded) {
                return Ok(());
            }
        }
        let result = self
            .call(
                "session/load",
                json!({
                    "sessionId": id,
                    "cwd": cwd,
                    "mcpServers": self.ask_mcp_servers(),
                    "_meta": crate::question::acp_session_meta(&self.permission_mode)
                }),
            )
            .await?;
        self.apply_session_result(id, cwd, &result).await;
        Ok(())
    }

    fn ask_mcp_servers(&self) -> Value {
        self.ask_bridge
            .as_ref()
            .map_or(json!([]), crate::question::mcp_ask_servers)
    }

    /// # Errors
    /// Returns an error if the session cannot be loaded or the prompt cannot be started.
    pub async fn prompt(
        &self,
        id: &str,
        cwd: &str,
        text: String,
        files: Vec<PromptFile>,
    ) -> Result<PromptOutcome> {
        let occ = self.occupancy_of(id, Some(cwd)).await;
        if !occ.writable {
            bail!(SESSION_BUSY);
        }
        self.yield_web_active(id).await;
        self.ensure().await?;
        let occ = self.occupancy_of(id, Some(cwd)).await;
        if !occ.writable {
            bail!(SESSION_BUSY);
        }
        self.session_load_inner(id, cwd).await?;
        let item = QueueItem {
            id: Uuid::new_v4().to_string(),
            text,
            files,
        };
        {
            let mut g = self.inner.lock().await;
            let sess = live_entry(&mut g, id, cwd);
            if sess.running {
                sess.queue.push_back(item);
                let queue: Vec<QueueItem> = sess.queue.iter().cloned().collect();
                drop(g);
                self.emit(id, "queue", &queue);
                return Ok(PromptOutcome {
                    queued: true,
                    queue,
                });
            }
        }
        self.start_prompt(id, item).await?;
        Ok(PromptOutcome {
            queued: false,
            queue: Vec::new(),
        })
    }

    /// # Errors
    /// Returns an error if the grok process is down or `session/cancel` cannot be sent.
    pub async fn cancel(&self, id: &str) -> Result<()> {
        let occ = self.occupancy_of(id, None).await;
        if occ.source != Source::Attached {
            bail!(SESSION_BUSY);
        }
        self.notify("session/cancel", json!({ "sessionId": id }))
            .await
    }

    pub async fn drop_session(&self, id: &str) {
        let mut g = self.inner.lock().await;
        g.sessions.remove(id);
        g.in_flight.retain(|_, sid| sid.as_str() != id);
    }

    async fn session_is_tui_held(&self, id: &str) -> bool {
        let s3 = ggok_core::occupy::cli_sessions(&self.grok_home);
        let our = ggok_core::occupy::our_runtime_pid(self.inner.lock().await.child_pid);
        ggok_core::occupy::tui_held(&s3, id, our)
    }

    async fn yield_web_active(&self, new_id: &str) {
        let prev = self.inner.lock().await.web_active_id.clone();
        if let Some(prev) = prev {
            let prev_is_tui = self.session_is_tui_held(&prev).await;
            if ggok_core::occupy::should_cancel_web_peer(&prev, new_id, prev_is_tui) {
                let occ = self.occupancy_of(&prev, None).await;
                if occ.source == Source::Attached && occ.running {
                    let _ = self
                        .notify("session/cancel", json!({ "sessionId": prev }))
                        .await;
                }
                self.drop_session(&prev).await;
            }
        }
        self.inner.lock().await.web_active_id = Some(new_id.to_string());
        let _ = ggok_core::occupy::write_web_active(
            &ggok_core::occupy::web_active_path(&self.leader_json),
            new_id,
        );
    }

    pub async fn queue_list(&self, id: &str) -> Vec<QueueItem> {
        let g = self.inner.lock().await;
        g.sessions
            .get(id)
            .map(|s| s.queue.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// # Errors
    /// Returns an error if the session is not loaded or the queue item is missing.
    pub async fn queue_patch(&self, id: &str, qid: &str, text: String) -> Result<Vec<QueueItem>> {
        self.require_attached(id).await?;
        let mut g = self.inner.lock().await;
        let Some(sess) = g.sessions.get_mut(id) else {
            bail!("session not loaded");
        };
        let Some(item) = sess.queue.iter_mut().find(|q| q.id == qid) else {
            bail!("queue item not found");
        };
        item.text = text;
        let queue: Vec<QueueItem> = sess.queue.iter().cloned().collect();
        drop(g);
        self.emit(id, "queue", &queue);
        Ok(queue)
    }

    /// # Errors
    /// Returns an error if the session is not loaded or the queue item is missing.
    pub async fn queue_delete(&self, id: &str, qid: &str) -> Result<Vec<QueueItem>> {
        self.require_attached(id).await?;
        let mut g = self.inner.lock().await;
        let Some(sess) = g.sessions.get_mut(id) else {
            bail!("session not loaded");
        };
        let before = sess.queue.len();
        sess.queue.retain(|q| q.id != qid);
        if sess.queue.len() == before {
            bail!("queue item not found");
        }
        let queue: Vec<QueueItem> = sess.queue.iter().cloned().collect();
        drop(g);
        self.emit(id, "queue", &queue);
        Ok(queue)
    }

    /// # Errors
    /// Returns an error if the session is not loaded or the prompt cannot be started.
    pub async fn queue_send_now(&self, id: &str, qid: &str) -> Result<Vec<QueueItem>> {
        self.require_attached(id).await?;
        let item;
        let running;
        let queue;
        {
            let mut g = self.inner.lock().await;
            let Some(sess) = g.sessions.get_mut(id) else {
                bail!("session not loaded");
            };
            let Some(pos) = sess.queue.iter().position(|q| q.id == qid) else {
                let rest: Vec<QueueItem> = sess.queue.iter().cloned().collect();
                return Ok(rest);
            };
            let Some(taken) = sess.queue.remove(pos) else {
                return Ok(sess.queue.iter().cloned().collect());
            };
            item = taken;
            running = sess.running;
            if running {
                sess.resume = Some(item.clone());
            }
            queue = sess.queue.iter().cloned().collect::<Vec<_>>();
            drop(g);
            self.emit(id, "queue", &queue);
        }
        if running {
            self.cancel(id).await?;
            return Ok(queue);
        }
        self.start_prompt(id, item).await?;
        Ok(self.queue_list(id).await)
    }

    /// # Errors
    /// Returns an error if the model or effort is invalid, or grok `session/set_model` fails.
    pub async fn set_model(
        &self,
        id: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<(String, String)> {
        self.require_attached(id).await?;
        {
            let g = self.inner.lock().await;
            validate_effort(&g.models, model, effort)?;
        }
        if self
            .call(
                "session/set_model",
                json!({ "sessionId": id, "modelId": model }),
            )
            .await
            .is_err()
        {
            let mut text = format!("/model {model}");
            if let Some(e) = effort.filter(|s| !s.is_empty()) {
                text.push(' ');
                text.push_str(e);
            }
            let _ = self
                .call(
                    "session/prompt",
                    json!({
                        "sessionId": id,
                        "prompt": [{ "type": "text", "text": text }]
                    }),
                )
                .await;
        }
        if let Some(effort) = effort.filter(|s| !s.is_empty()) {
            let _ = self
                .call(
                    "session/set_config_option",
                    json!({ "sessionId": id, "configId": effort, "value": true }),
                )
                .await;
        }
        let stored_effort = {
            let mut g = self.inner.lock().await;
            g.current_model = model.to_string();
            let default_effort = g
                .models
                .iter()
                .find(|m| m.id == model)
                .and_then(|m| m.effort.clone())
                .unwrap_or_default();
            if let Some(sess) = g.sessions.get_mut(id) {
                sess.model = model.to_string();
                if let Some(e) = effort.filter(|s| !s.is_empty()) {
                    sess.effort = e.to_string();
                } else if !default_effort.is_empty() {
                    sess.effort = default_effort;
                }
                sess.effort.clone()
            } else {
                effort.unwrap_or("").to_string()
            }
        };
        self.emit(
            id,
            "model",
            &json!({ "model": model, "effort": stored_effort }),
        );
        Ok((model.to_string(), stored_effort))
    }

    /// # Errors
    /// Returns an error if the session or permission request is missing, or stdin write fails.
    pub async fn answer_permission(&self, id: &str, req: &str, allow: bool) -> Result<()> {
        self.require_attached(id).await?;
        let mut g = self.inner.lock().await;
        let Some(sess) = g.sessions.get_mut(id) else {
            bail!("session not loaded");
        };
        let Some(pending) = sess.perms.remove(req) else {
            bail!("permission request not found");
        };
        let option_id = pick_option(&pending.options, allow)
            .ok_or_else(|| anyhow::anyhow!("no matching permission option"))?;
        let msg = json!({
            "jsonrpc": "2.0",
            "id": pending.rpc_id,
            "result": {
                "outcome": {
                    "outcome": "selected",
                    "optionId": option_id
                }
            }
        });
        write_stdin(g.stdin.as_mut(), &msg).await
    }

    pub(crate) async fn require_attached(&self, id: &str) -> Result<()> {
        let occ = self.occupancy_of(id, None).await;
        if occ.source != Source::Attached {
            bail!(SESSION_BUSY);
        }
        Ok(())
    }

    pub(crate) async fn start_prompt(&self, id: &str, item: QueueItem) -> Result<()> {
        let (cwd, image_ok, queue) = {
            let mut g = self.inner.lock().await;
            let image_ok = g.image_ok;
            let sess = live_entry(&mut g, id, "");
            sess.running = true;
            sess.user_emitted = true;
            reset_parser_keep_usage(sess);
            sess.parser.note_time(Some(now_ms()));
            (
                sess.cwd.clone(),
                image_ok,
                sess.queue.iter().cloned().collect::<Vec<_>>(),
            )
        };
        self.emit(id, "queue", &queue);
        self.emit_live(id, true);
        self.emit(
            id,
            "block",
            &Block::User {
                prompt_id: item.id.clone(),
                text: item.text.clone(),
                files: item.files.clone(),
            },
        );
        let prompt = build_prompt(&item.text, &item.files, &cwd, image_ok);
        let rpc_id = self
            .send(
                "session/prompt",
                json!({ "sessionId": id, "prompt": prompt }),
            )
            .await?;
        self.inner
            .lock()
            .await
            .in_flight
            .insert(rpc_id, id.to_string());
        Ok(())
    }

    pub(crate) async fn drain(&self, id: &str) {
        let next = {
            let mut g = self.inner.lock().await;
            let Some(sess) = g.sessions.get_mut(id) else {
                return;
            };
            if sess.running {
                return;
            }
            if let Some(item) = sess.resume.take() {
                Some(item)
            } else {
                sess.queue.pop_front()
            }
        };
        if let Some(item) = next {
            let _ = self.start_prompt(id, item).await;
        } else {
            let queue = self.queue_list(id).await;
            self.emit(id, "queue", &queue);
        }
    }

    pub(crate) async fn apply_initialize(&self, result: &Value) {
        let caps = result.get("agentCapabilities").unwrap_or(&Value::Null);
        let image_ok = caps
            .pointer("/promptCapabilities/image")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let meta = result.get("_meta").unwrap_or(&Value::Null);
        let (current, models) = parse_models(meta.get("modelState").unwrap_or(&Value::Null));
        let cmds = parse_commands(meta.get("availableCommands").unwrap_or(&Value::Null));
        let mut g = self.inner.lock().await;
        g.image_ok = image_ok;
        if !models.is_empty() {
            g.models = models;
        }
        if !current.is_empty() {
            g.current_model = current;
        }
        if !cmds.is_empty() {
            g.commands = cmds;
        }
    }

    pub(crate) async fn apply_session_result(&self, id: &str, cwd: &str, result: &Value) {
        let (current, models) = parse_models(result.get("models").unwrap_or(&Value::Null));
        let cmds = parse_commands(result.get("availableCommands").unwrap_or(&Value::Null));
        let mut g = self.inner.lock().await;
        if !models.is_empty() {
            g.models = models;
        }
        if !current.is_empty() {
            g.current_model.clone_from(&current);
        }
        if !cmds.is_empty() {
            g.commands = cmds;
        }
        let model = if current.is_empty() {
            g.current_model.clone()
        } else {
            current
        };
        let effort = g
            .models
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.effort.clone())
            .unwrap_or_default();
        let sess = live_entry(&mut g, id, cwd);
        sess.cwd = cwd.to_string();
        sess.loaded = true;
        if !model.is_empty() {
            sess.model = model;
        }
        if !effort.is_empty() && sess.effort.is_empty() {
            sess.effort = effort;
        }
    }

    pub(crate) async fn on_session_update(&self, params: &Value) {
        let sid = params
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if sid.is_empty() {
            return;
        }
        let update = params.get("update").cloned().unwrap_or(Value::Null);
        let kind = update
            .get("sessionUpdate")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if kind == "available_commands_update" {
            let cmds = parse_commands(update.get("availableCommands").unwrap_or(&Value::Null));
            self.inner.lock().await.commands.clone_from(&cmds);
            let merged = crate::slash::merge_commands(
                ggok_core::slash_docs::from_docs(&self.grok_home),
                cmds,
            );
            self.emit(&sid, "commands", &merged);
            return;
        }
        let meta = params.get("_meta").unwrap_or(&Value::Null);
        let prompt_id = meta
            .get("promptId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let ts = ggok_core::parse::timestamp_ms(params.get("timestamp")).or_else(|| Some(now_ms()));
        let (block, usage, emit_usage, before_ctx, ctx_used, model_id) = {
            let mut g = self.inner.lock().await;
            let sess = live_entry(&mut g, &sid, "");
            if kind == "user_message_chunk" && sess.user_emitted {
                sess.parser.ingest_at(&update, &prompt_id, ts);
                sess.parser.note_meta(meta);
                return;
            }
            let ingest = sess.parser.ingest_at(&update, &prompt_id, ts);
            let before_ctx = sess.parser.context_tokens();
            sess.parser.note_meta(meta);
            let skip_user = sess.user_emitted;
            let block = match ingest {
                Ingest::Text => {
                    let open = sess.parser.open_text();
                    if skip_user && matches!(open, Some(Block::User { .. })) {
                        None
                    } else {
                        open
                    }
                }
                Ingest::Tool => {
                    let id = update
                        .get("toolCallId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if !id.is_empty() {
                        sess.last_tool_id.clone_from(&id);
                    }
                    sess.parser.tool(&sess.last_tool_id)
                }
                Ingest::TurnEnd => sess.parser.last_block(),
                Ingest::Usage | Ingest::None => None,
            };
            let emit_usage = matches!(ingest, Ingest::TurnEnd | Ingest::Usage);
            if emit_usage {
                sess.usage = sess.parser.usage_snapshot();
            }
            let ctx_used = sess.parser.context_tokens();
            let model_id = sess.model.clone();
            let usage = sess.usage.clone();
            (block, usage, emit_usage, before_ctx, ctx_used, model_id)
        };
        let context = self.context_payload(ctx_used, before_ctx, &model_id).await;
        if let Some(block) = block {
            self.emit(&sid, "block", &block);
        }
        if emit_usage && usage.recorded {
            self.emit(&sid, "usage", &usage);
        }
        if let Some(context) = context {
            self.emit(&sid, "context", &context);
        }
        self.maybe_present_ask_tool(&sid, &kind, update).await;
    }

    async fn context_payload(
        &self,
        ctx_used: u64,
        before_ctx: u64,
        model_id: &str,
    ) -> Option<Value> {
        if ctx_used == before_ctx {
            return None;
        }
        let window = {
            let g = self.inner.lock().await;
            g.models
                .iter()
                .find(|m| m.id == model_id)
                .and_then(|m| m.context_window)
                .filter(|n| *n > 0)
                .unwrap_or_else(|| ggok_core::parse::context_window(&self.grok_home, model_id))
        };
        Some(json!({ "used": ctx_used, "window": window }))
    }

    async fn maybe_present_ask_tool(&self, sid: &str, kind: &str, update: Value) {
        if kind != "tool_call" && kind != "tool_call_update" {
            return;
        }
        let title = update.get("title").and_then(Value::as_str).unwrap_or("");
        let name = update
            .pointer("/_meta/x.ai/tool/name")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !crate::question::should_present_tool_call_as_ask(title, name, &update) {
            return;
        }
        tracing::info!(
            session_id = %sid,
            %kind,
            %title,
            %name,
            "ask_user_question tool_call"
        );
        let fallback = {
            let g = self.inner.lock().await;
            g.sessions
                .get(sid)
                .map(|sess| sess.last_tool_id.clone())
                .unwrap_or_default()
        };
        let params = crate::question::with_fallback_tool_id(update, &fallback);
        self.handle_ask_user(Value::Null, params).await;
    }

    pub(crate) async fn refresh_usage_from_disk(&self, sid: &str) {
        let cwd = {
            let g = self.inner.lock().await;
            let Some(sess) = g.sessions.get(sid) else {
                return;
            };
            sess.cwd.clone()
        };
        if cwd.is_empty() {
            return;
        }
        let path = self
            .grok_home
            .join("sessions")
            .join(percent_encode_cwd(&cwd))
            .join(sid)
            .join("updates.jsonl");
        let Ok(parsed) = ggok_core::parse::parse_updates_file(&path) else {
            return;
        };
        if !parsed.usage.recorded {
            return;
        }
        let model = {
            let mut g = self.inner.lock().await;
            let Some(sess) = g.sessions.get_mut(sid) else {
                return;
            };
            sess.parser.replace_usage(parsed.usage.clone());
            sess.usage = parsed.usage.clone();
            sess.parser.set_context_tokens(parsed.context_tokens);
            sess.model.clone()
        };
        self.emit(sid, "usage", &parsed.usage);
        if parsed.context_tokens > 0 {
            let window = ggok_core::parse::context_window(&self.grok_home, &model);
            self.emit(
                sid,
                "context",
                &json!({ "used": parsed.context_tokens, "window": window }),
            );
        }
    }
}

fn validate_effort(models: &[ModelInfo], model: &str, effort: Option<&str>) -> Result<()> {
    let Some(effort) = effort.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let Some(info) = models.iter().find(|m| m.id == model) else {
        return Ok(());
    };
    if info.efforts.iter().any(|e| e.id == effort) {
        return Ok(());
    }
    bail!("invalid effort");
}

pub(crate) fn pick_option(options: &[PermOpt], allow: bool) -> Option<String> {
    if allow {
        options
            .iter()
            .find(|o| o.kind.contains("allow") && !o.kind.contains("always"))
            .or_else(|| options.iter().find(|o| o.kind.contains("allow")))
            .or_else(|| options.first())
            .map(|o| o.id.clone())
    } else {
        options
            .iter()
            .find(|o| o.kind.contains("reject") || o.kind.contains("deny"))
            .or_else(|| options.last())
            .map(|o| o.id.clone())
    }
}

pub(crate) fn parse_models(v: &Value) -> (String, Vec<ModelInfo>) {
    let current = v
        .get("currentModelId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let models = v
        .get("availableModels")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let id = m
                        .get("modelId")
                        .or_else(|| m.get("id"))
                        .and_then(Value::as_str)?
                        .to_string();
                    let name = m
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(&id)
                        .to_string();
                    let description = m
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let meta = m.get("_meta").unwrap_or(&Value::Null);
                    let context_window = m
                        .get("contextWindow")
                        .or_else(|| m.get("context_window"))
                        .or_else(|| meta.get("contextWindow"))
                        .or_else(|| meta.get("context_window"))
                        .and_then(|v| {
                            v.as_u64()
                                .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
                        })
                        .filter(|n| *n > 0);
                    let effort = meta
                        .get("reasoningEffort")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                    let efforts = meta
                        .get("reasoningEfforts")
                        .and_then(Value::as_array)
                        .map(|e| {
                            e.iter()
                                .filter_map(|x| {
                                    Some(EffortInfo {
                                        id: x.get("id")?.as_str()?.to_string(),
                                        label: x
                                            .get("label")
                                            .and_then(Value::as_str)
                                            .unwrap_or("")
                                            .to_string(),
                                        description: x
                                            .get("description")
                                            .and_then(Value::as_str)
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(ModelInfo {
                        id,
                        name,
                        description,
                        effort,
                        efforts,
                        context_window,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (current, models)
}

pub(crate) fn parse_commands(v: &Value) -> Vec<SlashCommand> {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let name = c.get("name")?.as_str()?.to_string();
                    Some(SlashCommand {
                        name,
                        description: c
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        hint: c
                            .get("input")
                            .and_then(|i| i.get("hint"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        aliases: c
                            .get("aliases")
                            .and_then(Value::as_array)
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(ToOwned::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_prompt(text: &str, files: &[PromptFile], cwd: &str, image_ok: bool) -> Value {
    let mut parts = Vec::new();
    let mut body = text.to_string();
    let cwd_path = Path::new(cwd);
    for f in files {
        let path = Path::new(&f.path);
        let mime = f.mime.as_deref().unwrap_or("");
        if image_ok
            && mime.starts_with("image/")
            && let Ok(bytes) = fs::read(path)
        {
            parts.push(json!({
                "type": "image",
                "mimeType": mime,
                "data": STANDARD.encode(bytes)
            }));
            continue;
        }
        let rel = path
            .strip_prefix(cwd_path)
            .map_or_else(|_| f.path.clone(), |p| p.to_string_lossy().into_owned());
        let tag = format!("@{rel}");
        if !body.contains(&tag) {
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&tag);
        }
    }
    let mut out = vec![json!({ "type": "text", "text": body })];
    out.append(&mut parts);
    Value::Array(out)
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

fn percent_encode_cwd(cwd: &str) -> String {
    percent_encoding::utf8_percent_encode(cwd, percent_encoding::NON_ALPHANUMERIC).to_string()
}
