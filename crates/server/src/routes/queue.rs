use crate::http::{json_ok, valid_id};
use crate::service::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

pub(crate) async fn api_queue(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    json_ok(&state.agent.queue_list(&id).await)
}

#[derive(Debug, Deserialize)]
pub(crate) struct QueuePatch {
    pub text: String,
}

pub(crate) async fn api_queue_patch(
    State(state): State<Arc<AppState>>,
    Path((id, qid)): Path<(String, String)>,
    Json(body): Json<QueuePatch>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    match state.agent.queue_patch(&id, &qid, body.text).await {
        Ok(q) => json_ok(&q),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub(crate) async fn api_queue_delete(
    State(state): State<Arc<AppState>>,
    Path((id, qid)): Path<(String, String)>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    match state.agent.queue_delete(&id, &qid).await {
        Ok(q) => json_ok(&q),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub(crate) async fn api_queue_send(
    State(state): State<Arc<AppState>>,
    Path((id, qid)): Path<(String, String)>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    match state.agent.queue_send_now(&id, &qid).await {
        Ok(q) => json_ok(&q),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
