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
    pub allow: bool,
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
    match state.agent.answer_permission(&id, &req, body.allow).await {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
