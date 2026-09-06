use crate::http::{json_ok, valid_id};
use crate::service::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ggok_agent::QuestionReply;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub(crate) struct AskBody {
    #[serde(default)]
    session_id: String,
    questions: Value,
}

pub(crate) async fn api_ask_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AskBody>,
) -> Response {
    match state
        .agent
        .present_web_question(Some(body.session_id.as_str()), body.questions)
        .await
    {
        Ok((id, req)) => json_ok(&json!({ "session_id": id, "req": req })),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub(crate) async fn api_ask_wait(
    State(state): State<Arc<AppState>>,
    Path(req): Path<String>,
) -> Response {
    if req.is_empty() {
        return (StatusCode::BAD_REQUEST, "invalid question id").into_response();
    }
    match state.agent.wait_web_question(&req).await {
        Ok(reply) => json_ok(&reply),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub(crate) async fn api_question(
    State(state): State<Arc<AppState>>,
    Path((id, req)): Path<(String, String)>,
    Json(body): Json<QuestionReply>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    if req.is_empty() {
        return (StatusCode::BAD_REQUEST, "invalid question id").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    if ggok_core::occupy::conflict_busy(occ, ggok_core::occupy::SessionOp::Control) {
        return super::session_busy();
    }
    match state.agent.answer_question(&id, &req, body).await {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => {
            let msg = e.to_string();
            if msg == ggok_core::occupy::SESSION_BUSY {
                super::session_busy()
            } else {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
        }
    }
}
