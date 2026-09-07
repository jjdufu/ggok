use crate::http::{json_ok, valid_id};
use crate::service::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ggok_core::types::PromptFile;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub(crate) struct PromptBody {
    pub text: String,
    #[serde(default)]
    pub files: Vec<PromptFile>,
}

pub(crate) async fn api_prompt(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<PromptBody>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let cwd = match state.session(&id) {
        Some(m) => m.cwd,
        None => {
            return (StatusCode::NOT_FOUND, "session not found").into_response();
        }
    };
    let occ = super::occupancy(&state, &id).await;
    if ggok_core::occupy::conflict_busy(occ, ggok_core::occupy::SessionOp::Prompt) {
        return super::session_busy();
    }
    match state.agent.prompt(&id, &cwd, body.text, body.files).await {
        Ok(out) => json_ok(&out),
        Err(e) => super::map_agent_err(&e),
    }
}

pub(crate) async fn api_interject(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<PromptBody>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let cwd = match state.session(&id) {
        Some(m) => m.cwd,
        None => {
            return (StatusCode::NOT_FOUND, "session not found").into_response();
        }
    };
    let occ = super::occupancy(&state, &id).await;
    if ggok_core::occupy::conflict_busy(occ, ggok_core::occupy::SessionOp::Prompt) {
        return super::session_busy();
    }
    let item = ggok_core::types::QueueItem {
        id: uuid::Uuid::new_v4().to_string(),
        text: body.text,
        files: body.files,
    };
    match state.agent.session_load(&id, &cwd).await {
        Ok(()) => {}
        Err(e) => return super::map_agent_err(&e),
    }
    match state.agent.interject(&id, item).await {
        Ok(out) => json_ok(&out),
        Err(e) => super::map_agent_err(&e),
    }
}

pub(crate) async fn api_cancel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    if ggok_core::occupy::conflict_busy(occ, ggok_core::occupy::SessionOp::Cancel) {
        return super::session_busy();
    }
    match state.agent.cancel(&id).await {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => super::map_agent_err(&e),
    }
}
