use crate::account;
use crate::host;
use crate::http::json_ok;
use crate::service::AppState;
use axum::extract::State;
use axum::response::Response;
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn api_runtime(State(state): State<Arc<AppState>>) -> Response {
    let view = state.agent.runtime().await;
    json_ok(&json!({
        "agent_ok": view.agent_ok,
        "models": view.models,
        "current_model": view.current_model,
        "workspace_roots": state.workspace_roots.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
        "permission_mode": state.permission_mode,
        "commands": view.commands,
        "poll_secs": state.poll_secs,
        "upload_max_bytes": state.upload_max_bytes,
        "user": host::current_user(),
        "email": account::local_email(&state.grok_home),
    }))
}

pub(crate) async fn api_status(State(state): State<Arc<AppState>>) -> Response {
    json_ok(&host::snapshot(&state.grok_home))
}

pub(crate) async fn api_account(State(state): State<Arc<AppState>>) -> Response {
    json_ok(&account::snapshot(&state.grok_home).await)
}
