use crate::http::{json_ok, valid_id};
use crate::service::AppState;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use ggok_agent::session_ops::delete_session;
use ggok_core::occupy::{self, Source};
use ggok_core::parse::{blocks_to_markdown, extract_tool};
use ggok_core::paths;
use ggok_core::scan;
use ggok_core::search::search_session_ids;
use ggok_core::session;
use ggok_core::types::{ContextUse, SessionDetail, SessionMeta, ToolDetail};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Deserialize)]
pub(crate) struct ListQuery {
    pub cwd: Option<String>,
    pub q: Option<String>,
    pub empty: Option<u8>,
}

pub(crate) async fn api_sessions(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> Response {
    let include_empty = q.empty.unwrap_or(0) != 0;
    let fts =
        q.q.as_deref()
            .filter(|s| !s.is_empty())
            .and_then(|s| search_session_ids(&state.grok_home, s));
    let mut rows = state.sessions.read().list(
        q.cwd.as_deref().filter(|s| !s.is_empty()),
        q.q.as_deref(),
        include_empty,
        fts.as_deref(),
    );
    let live = state.agent.live_map().await;
    let our = occupy::our_runtime_pid(state.agent.child_pid().await);
    let leftover = occupy::leftover_noleader_pid(&state.agent_pid_file).is_some();
    let s3 = occupy::cli_sessions(&state.grok_home);
    let pins = session::load_pins(&state.pins_path);
    let index = state.sessions.read();
    for row in &mut rows {
        let jsonl = index
            .get(&row.id)
            .is_some_and(|m| occupy::jsonl_running(&m.dir));
        let occ = occupy::classify(&occupy::ClassifyInput {
            id: &row.id,
            live: live.get(&row.id),
            our_runtime_pid: our,
            s3: &s3,
            leftover_noleader_alive: leftover,
            jsonl_running: jsonl,
        });
        row.running = occ.running;
        row.source = occ.source.as_str().to_string();
        row.pinned = pins.iter().any(|id| id == &row.id);
    }
    json_ok(&rows)
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateSession {
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
}

pub(crate) async fn api_create_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateSession>,
) -> Response {
    let cwd = match paths::cwd_allowed(&body.cwd, &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    match state
        .agent
        .session_new(&cwd, body.model.as_deref(), body.effort.as_deref())
        .await
    {
        Ok(s) => {
            insert_stub(&state, &s.id, &s.cwd, &s.model);
            let effort = state
                .agent
                .live_view(&s.id)
                .await
                .and_then(|l| {
                    if l.effort.is_empty() {
                        None
                    } else {
                        Some(l.effort)
                    }
                })
                .or(body.effort);
            json_ok(&json!({ "id": s.id, "cwd": s.cwd, "model": s.model, "effort": effort }))
        }
        Err(e) => super::map_agent_err(&e),
    }
}

pub(crate) async fn api_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let (id, as_md) = match id.strip_suffix(".md") {
        Some(stripped) => (stripped.to_string(), true),
        None => (id, false),
    };
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let Some(meta) = state.session(&id) else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    };
    let parsed = match state.parsed(&meta) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("parse {}: {e:#}", meta.id);
            return (StatusCode::INTERNAL_SERVER_ERROR, "parse failed").into_response();
        }
    };
    if as_md {
        let md = blocks_to_markdown(&parsed.blocks);
        let filename = format!("{id}.md");
        return Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/markdown; charset=utf-8")
            .header(
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            )
            .body(Body::from(md))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    let live = state.agent.live_view(&id).await;
    let occ = super::occupancy(&state, &id).await;
    let model = live
        .as_ref()
        .and_then(|l| {
            if l.model.is_empty() {
                None
            } else {
                Some(l.model.clone())
            }
        })
        .unwrap_or(meta.model);
    let effort = live.as_ref().and_then(|l| {
        if l.effort.is_empty() {
            None
        } else {
            Some(l.effort.clone())
        }
    });
    let window = ggok_core::parse::context_window(&state.grok_home, &model);
    let live_usage = state.agent.live_usage(&id).await;
    let (usage, ctx_used) = match live_usage {
        Some((live_u, live_ctx))
            if live_u.recorded
                && (!parsed.usage.recorded || live_u.total_tokens >= parsed.usage.total_tokens) =>
        {
            (live_u, live_ctx.max(parsed.context_tokens))
        }
        Some((_, live_ctx)) => (parsed.usage.clone(), live_ctx.max(parsed.context_tokens)),
        None => (parsed.usage.clone(), parsed.context_tokens),
    };
    let work_started_ms = parsed
        .work_started_ms
        .or(state.agent.live_work_started_ms(&id).await);
    json_ok(&SessionDetail {
        id: meta.id,
        cwd: meta.cwd,
        title: meta.title,
        model,
        effort,
        source: occ.source.as_str().to_string(),
        writable: occ.writable,
        running: occ.running,
        blocks: {
            if occ.running {
                let live_blocks = state.agent.live_blocks(&id).await;
                if live_blocks.is_empty() {
                    parsed.blocks.clone()
                } else {
                    ggok_core::parse::merge_live_over_disk(&parsed.blocks, &live_blocks)
                }
            } else {
                parsed.blocks.clone()
            }
        },
        usage,
        context: ContextUse {
            used: ctx_used,
            window,
        },
        work_started_ms,
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct SessionPatch {
    pub title: Option<String>,
    pub pinned: Option<bool>,
}

pub(crate) async fn api_patch_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SessionPatch>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let Some(meta) = state.session(&id) else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    };
    if body.title.is_none() && body.pinned.is_none() {
        return (StatusCode::BAD_REQUEST, "title or pinned required").into_response();
    }
    if let Some(title) = body.title.as_deref()
        && let Err(e) = session::rename_summary(&meta.dir, &id, &meta.cwd, title)
    {
        return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    if let Some(pinned) = body.pinned
        && let Err(e) = session::set_pinned(&state.pins_path, &id, pinned)
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Ok(next) = scan::scan(&state.grok_home) {
        state.replace_sessions(next);
    } else if let Some(title) = body.title.as_deref()
        && let Some(m) = state.sessions.write().sessions.get_mut(&id)
    {
        m.title = title.trim().to_string();
    }
    let title = state.session(&id).map_or_else(|| id.clone(), |m| m.title);
    let pinned = session::is_pinned(&state.pins_path, &id);
    json_ok(&json!({ "id": id, "title": title, "pinned": pinned }))
}

pub(crate) async fn api_delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    if state.session(&id).is_none() {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    if occupy::conflict_busy(occ, occupy::SessionOp::Delete) {
        return super::session_busy();
    }
    if occ.source == Source::Attached
        && occ.running
        && let Err(e) = state.agent.cancel(&id).await
    {
        tracing::warn!("cancel before delete {id}: {e:#}");
    }
    if occ.source == Source::Attached {
        state.agent.drop_session(&id).await;
    }
    match delete_session(&state.grok_bin, &state.grok_home, &id).await {
        Ok(()) => {
            let _ = session::set_pinned(&state.pins_path, &id, false);
            if let Ok(next) = scan::scan(&state.grok_home) {
                state.replace_sessions(next);
            } else {
                state.sessions.write().sessions.remove(&id);
            }
            json_ok(&json!({ "ok": true, "id": id }))
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

pub(crate) async fn api_tool(
    State(state): State<Arc<AppState>>,
    Path((id, tool_id)): Path<(String, String)>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    if tool_id.contains("..") || tool_id.contains('/') {
        return (StatusCode::BAD_REQUEST, "invalid tool id").into_response();
    }
    let Some(meta) = state.session(&id) else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    };
    let path = meta.dir.join("updates.jsonl");
    let tool_id = tool_id.clone();
    match tokio::task::spawn_blocking(move || extract_tool(&path, &tool_id)).await {
        Ok(Ok(Some(detail))) => json_ok(&detail),
        Ok(Ok(None)) => json_ok(&ToolDetail {
            content: serde_json::Value::Null,
            raw_output: serde_json::Value::Null,
            log: String::new(),
        }),
        Ok(Err(_)) | Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "parse failed").into_response(),
    }
}

pub(crate) async fn api_load(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let Some(meta) = state.session(&id) else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    };
    let occ = super::occupancy(&state, &id).await;
    if occupy::conflict_busy(occ, occupy::SessionOp::Load) {
        return super::session_busy();
    }
    match state.agent.session_load(&id, &meta.cwd).await {
        Ok(()) => {
            let live = state.agent.live_view(&id).await;
            json_ok(&json!({
                "id": id,
                "cwd": meta.cwd,
                "model": live.as_ref().map(|l| l.model.clone()).unwrap_or_default(),
                "effort": live.as_ref().map(|l| l.effort.clone()).unwrap_or_default(),
            }))
        }
        Err(e) => super::map_agent_err(&e),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelBody {
    pub model: String,
    pub effort: Option<String>,
}

pub(crate) async fn api_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ModelBody>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    if occupy::conflict_busy(occ, occupy::SessionOp::Control) {
        return super::session_busy();
    }
    match state
        .agent
        .set_model(&id, &body.model, body.effort.as_deref())
        .await
    {
        Ok((model, effort)) => json_ok(&json!({ "model": model, "effort": effort })),
        Err(e) => super::map_agent_err(&e),
    }
}

fn insert_stub(state: &AppState, id: &str, cwd: &str, model: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let meta = SessionMeta {
        id: id.to_string(),
        cwd: cwd.to_string(),
        title: id.to_string(),
        created_at: now.clone(),
        updated_at: now,
        updated_sort: i64::try_from(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        )
        .unwrap_or(0),
        model: model.to_string(),
        agent_name: String::new(),
        num_messages: 0,
        parent_id: None,
        empty: true,
        dir: state
            .grok_home
            .join("sessions")
            .join(percent_encode(cwd))
            .join(id),
    };
    if let Ok(mut next) = scan::scan(&state.grok_home) {
        next.sessions.entry(id.to_string()).or_insert(meta);
        state.replace_sessions(next);
    } else {
        state.sessions.write().sessions.insert(id.to_string(), meta);
    }
}

fn percent_encode(cwd: &str) -> String {
    percent_encoding::utf8_percent_encode(cwd, percent_encoding::NON_ALPHANUMERIC).to_string()
}
