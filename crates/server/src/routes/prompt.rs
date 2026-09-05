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
    match state.agent.prompt(&id, &cwd, body.text, body.files).await {
        Ok(out) => json_ok(&out),
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

pub(crate) async fn api_cancel(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    match state.agent.cancel(&id).await {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}
