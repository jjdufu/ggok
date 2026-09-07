use crate::http::{json_ok, valid_id};
use crate::service::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub(crate) struct PermBody {
    pub allow: Option<bool>,
    pub option_id: Option<String>,
    pub message: Option<String>,
}

pub(crate) async fn api_permission(
    State(state): State<Arc<AppState>>,
    Path((id, req)): Path<(String, String)>,
    Json(body): Json<PermBody>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    if ggok_core::occupy::conflict_busy(occ, ggok_core::occupy::SessionOp::Control) {
        return super::session_busy();
    }
    let result = if let Some(option_id) = body
        .option_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        state
            .agent
            .answer_permission_option(&id, &req, option_id)
            .await
    } else if let Some(allow) = body.allow {
        state.agent.answer_permission(&id, &req, allow).await
    } else {
        return (StatusCode::BAD_REQUEST, "option_id or allow required").into_response();
    };
    match result {
        Ok(()) => {
            if let Some(msg) = body
                .message
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                && body.allow == Some(false)
            {
                let cwd = state.session(&id).map(|m| m.cwd).unwrap_or_default();
                if !cwd.is_empty() {
                    let _ = state
                        .agent
                        .prompt(&id, &cwd, msg.to_string(), Vec::new())
                        .await;
                }
            }
            json_ok(&json!({ "ok": true }))
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
