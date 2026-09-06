use super::{Agent, PendingPerm, PermOpt, live_entry};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::oneshot;

impl Agent {
    pub(crate) async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        {
            let mut g = self.inner.lock().await;
            let id = g.next_id;
            g.next_id += 1;
            g.pending.insert(id, tx);
            let msg = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            });
            write_stdin(g.stdin.as_mut(), &msg).await?;
        }
        match rx.await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => bail!("{method}: {e}"),
            Err(_) => bail!("{method}: grok agent closed"),
        }
    }

    pub(crate) async fn send(&self, method: &str, params: Value) -> Result<u64> {
        let mut g = self.inner.lock().await;
        let id = g.next_id;
        g.next_id += 1;
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        write_stdin(g.stdin.as_mut(), &msg).await?;
        Ok(id)
    }

    pub(crate) async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let mut g = self.inner.lock().await;
        let msg = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        write_stdin(g.stdin.as_mut(), &msg).await
    }

    pub(crate) async fn on_message(&self, msg: Value) -> Result<()> {
        if msg.get("method").is_some() && msg.get("id").is_some() {
            self.on_request(&msg).await;
            return Ok(());
        }
        if msg.get("method").is_some() {
            self.on_note(&msg).await;
            return Ok(());
        }
        if let Some(id) = json_id_u64(msg.get("id")) {
            let result = if let Some(err) = msg.get("error") {
                Err(err.to_string())
            } else {
                Ok(msg.get("result").cloned().unwrap_or(Value::Null))
            };
            let sid = {
                let mut g = self.inner.lock().await;
                if let Some(tx) = g.pending.remove(&id) {
                    let _ = tx.send(result.clone());
                }
                g.in_flight.remove(&id)
            };
            if let Some(sid) = sid {
                let usage = {
                    let mut g = self.inner.lock().await;
                    if let Some(sess) = g.sessions.get_mut(&sid) {
                        sess.running = false;
                        sess.user_emitted = false;
                        sess.usage = sess.parser.usage_snapshot();
                        Some(sess.usage.clone())
                    } else {
                        None
                    }
                };
                if let Some(usage) = usage.filter(|u| u.recorded) {
                    self.emit(&sid, "usage", &usage);
                }
                self.refresh_usage_from_disk(&sid).await;
                if let Err(e) = &result {
                    self.emit(&sid, "error", &json!({ "message": e }));
                }
                self.clear_questions(&sid).await;
                self.emit_live(&sid, false);
                self.emit(&sid, "done", &json!({}));
                let agent = self.clone();
                tokio::spawn(async move {
                    agent.drain(&sid).await;
                });
            }
        }
        Ok(())
    }

    pub(crate) async fn on_request(&self, msg: &Value) {
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        tracing::info!(%method, id = %id, "acp client request");
        let (method, params) = crate::question::unwrap_ext_request(method, params);
        if crate::question::is_ask_user_method(&method) {
            self.handle_ask_user(id, params).await;
            return;
        }
        if method == "session/request_permission" {
            self.handle_permission(id, params).await;
            return;
        }
        if crate::question::looks_like_ask_user(&params) {
            tracing::info!(%method, "treating ACP request as ask_user_question");
            self.handle_ask_user(id, params).await;
            return;
        }
        tracing::warn!(%method, params = %params, "acp client method not found");
        let mut g = self.inner.lock().await;
        let reply = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": "Method not found" }
        });
        let _ = write_stdin(g.stdin.as_mut(), &reply).await;
    }

    async fn handle_permission(&self, id: Value, params: Value) {
        let sid = params
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let tool = params.get("toolCall").cloned().unwrap_or(Value::Null);
        let tool_id = tool
            .get("toolCallId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let title = tool
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let options: Vec<PermOpt> = params
            .get("options")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|o| {
                        Some(PermOpt {
                            id: o.get("optionId")?.as_str()?.to_string(),
                            kind: o
                                .get("kind")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            name: o
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let req_key = json_id_key(&id);
        {
            let mut g = self.inner.lock().await;
            if let Some(sess) = g.sessions.get_mut(&sid) {
                sess.perms.insert(
                    req_key.clone(),
                    PendingPerm {
                        rpc_id: id,
                        options: options.clone(),
                    },
                );
            }
        }
        self.emit(
            &sid,
            "permission",
            &json!({
                "req": req_key,
                "tool_id": tool_id,
                "title": title,
                "options": options.iter().map(|o| json!({
                    "id": o.id, "kind": o.kind, "name": o.name
                })).collect::<Vec<_>>()
            }),
        );
    }

    pub(crate) async fn on_note(&self, msg: &Value) {
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").unwrap_or(&Value::Null);
        match method {
            "session/update" | "_x.ai/session/update" => self.on_session_update(params).await,
            "_x.ai/queue/changed" => self.on_queue_changed(params).await,
            method
                if crate::question::is_ask_user_method(method)
                    || crate::question::looks_like_ask_user(params) =>
            {
                tracing::info!(%method, "acp ask_user_question notification");
                self.handle_ask_user(Value::Null, params.clone()).await;
            }
            "_x.ai/models/update" => {
                let (current, models) = super::session::parse_models(params);
                let sid = params
                    .get("sessionId")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                {
                    let mut g = self.inner.lock().await;
                    if !current.is_empty() {
                        g.current_model.clone_from(&current);
                    }
                    if !models.is_empty() {
                        g.models = models;
                    }
                    if !sid.is_empty()
                        && !current.is_empty()
                        && let Some(sess) = g.sessions.get_mut(&sid)
                    {
                        sess.model.clone_from(&current);
                    }
                }
                if !sid.is_empty() && !current.is_empty() {
                    let effort = {
                        let g = self.inner.lock().await;
                        g.sessions
                            .get(&sid)
                            .map(|s| s.effort.clone())
                            .or_else(|| {
                                g.models
                                    .iter()
                                    .find(|m| m.id == current)
                                    .and_then(|m| m.effort.clone())
                            })
                            .unwrap_or_default()
                    };
                    self.emit(
                        &sid,
                        "model",
                        &json!({ "model": current, "effort": effort }),
                    );
                }
            }
            other => {
                tracing::debug!(method = other, "acp notification");
            }
        }
    }

    async fn on_queue_changed(&self, params: &Value) {
        let sid = params
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if sid.is_empty() {
            return;
        }
        let items = params
            .get("queue")
            .or_else(|| params.get("items"))
            .cloned()
            .unwrap_or(Value::Null);
        let parsed: Vec<ggok_core::types::QueueItem> =
            serde_json::from_value(items).unwrap_or_default();
        {
            let mut g = self.inner.lock().await;
            let sess = live_entry(&mut g, &sid, "");
            sess.queue = parsed.iter().cloned().collect();
        }
        self.emit(&sid, "queue", &parsed);
    }
}

pub(crate) async fn write_stdin(stdin: Option<&mut ChildStdin>, msg: &Value) -> Result<()> {
    let Some(stdin) = stdin else {
        bail!("grok agent is not running");
    };
    let mut line = serde_json::to_vec(msg)?;
    line.push(b'\n');
    stdin.write_all(&line).await?;
    stdin.flush().await?;
    Ok(())
}

pub(crate) fn json_id_u64(id: Option<&Value>) -> Option<u64> {
    let id = id?;
    id.as_u64()
        .or_else(|| id.as_i64().and_then(|n| u64::try_from(n).ok()))
        .or_else(|| id.as_str().and_then(|s| s.parse().ok()))
}

pub(crate) fn json_id_key(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}
