use crate::account;
use crate::host;
use crate::http::json_ok;
use crate::service::AppState;
use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn api_runtime(State(state): State<Arc<AppState>>) -> Response {
    let view = state.agent.runtime().await;
    let last = ggok_core::config::config_dir()
        .ok()
        .map(|dir| ggok_core::prefs::load_last_model(&ggok_core::prefs::last_model_path(&dir)))
        .unwrap_or_default();
    json_ok(&json!({
        "agent_ok": view.agent_ok,
        "models": view.models,
        "current_model": view.current_model,
        "last_model": last.model,
        "last_effort": last.effort,
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

pub(crate) async fn api_version() -> Response {
    json_ok(&crate::release::snapshot())
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct PrefsModelBody {
    pub model: String,
    pub effort: Option<String>,
}

pub(crate) async fn api_prefs_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PrefsModelBody>,
) -> Response {
    let model = body.model.trim();
    if model.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "model required").into_response();
    }
    let effort = body.effort.as_deref().unwrap_or("").trim();
    ggok_core::prefs::remember_model_choice(&state.grok_home, model, effort);
    json_ok(&json!({ "model": model, "effort": effort }))
}
