pub(crate) mod control;
pub(crate) mod events;
pub(crate) mod ext;
pub(crate) mod fs;
pub(crate) mod meta;
pub(crate) mod perm;
pub(crate) mod prompt;
pub(crate) mod question;
pub(crate) mod queue;
pub(crate) mod session;
pub(crate) mod workspace;

use crate::service::AppState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use ggok_core::occupy::{Occupancy, SESSION_BUSY};
use std::sync::Arc;

pub(crate) async fn occupancy(state: &AppState, id: &str) -> Occupancy {
    let cwd = state.session(id).map(|m| m.cwd);
    state.agent.occupancy_of(id, cwd.as_deref()).await
}

pub(crate) fn session_busy() -> Response {
    (StatusCode::CONFLICT, SESSION_BUSY).into_response()
}

pub(crate) fn map_agent_err(err: &anyhow::Error) -> Response {
    let msg = err.to_string();
    if msg == SESSION_BUSY {
        session_busy()
    } else if msg.contains("invalid effort") || msg.contains("invalid mode") {
        (StatusCode::BAD_REQUEST, msg).into_response()
    } else if msg.starts_with(ggok_agent::ACP_UNAVAILABLE)
        || msg.contains("worktree fork is not implemented")
    {
        (StatusCode::NOT_IMPLEMENTED, msg).into_response()
    } else {
        (StatusCode::BAD_GATEWAY, msg).into_response()
    }
}

pub(crate) fn router(upload_max: usize) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/runtime", get(meta::api_runtime))
        .route("/api/prefs/model", post(meta::api_prefs_model))
        .route("/api/dirs", get(fs::api_dirs))
        .route("/api/status", get(meta::api_status))
        .route("/api/account", get(meta::api_account))
        .route("/api/version", get(meta::api_version))
        .route("/api/fs", get(fs::api_fs))
        .route(
            "/api/workspace",
            get(workspace::api_workspace_list).delete(workspace::api_workspace_delete),
        )
        .route("/api/workspace/file", get(workspace::api_workspace_file))
        .route(
            "/api/workspace/archive",
            get(workspace::api_workspace_archive),
        )
        .route("/api/mcp", get(ext::api_mcp_get).post(ext::api_mcp_post))
        .route(
            "/api/plugins",
            get(ext::api_plugins_get).post(ext::api_plugins_post),
        )
        .route(
            "/api/skills",
            get(ext::api_skills_get).post(ext::api_skills_post),
        )
        .route("/api/skills/item", get(ext::api_skills_item))
        .route("/api/skills/upload", post(ext::api_skills_upload))
        .route(
            "/api/uploads",
            get(fs::api_upload_get).post(fs::api_uploads),
        )
        .route(
            "/api/sessions",
            get(session::api_sessions).post(session::api_create_session),
        )
        .route(
            "/api/sessions/{id}",
            get(session::api_session)
                .patch(session::api_patch_session)
                .delete(session::api_delete_session),
        )
        .route("/api/sessions/{id}/tools/{tool_id}", get(session::api_tool))
        .route("/api/sessions/{id}/load", post(session::api_load))
        .route("/api/sessions/{id}/prompt", post(prompt::api_prompt))
        .route("/api/sessions/{id}/cancel", post(prompt::api_cancel))
        .route("/api/sessions/{id}/interject", post(prompt::api_interject))
        .route("/api/sessions/{id}/model", post(session::api_model))
        .route(
            "/api/sessions/{id}/rewind/points",
            get(control::api_rewind_points),
        )
        .route("/api/sessions/{id}/rewind", post(control::api_rewind))
        .route("/api/sessions/{id}/retry", post(control::api_retry))
        .route("/api/sessions/{id}/fork", post(control::api_fork))
        .route("/api/sessions/{id}/compact", post(control::api_compact))
        .route("/api/sessions/{id}/mode", post(control::api_mode))
        .route("/api/sessions/{id}/plan", get(control::api_plan))
        .route("/api/sessions/{id}/btw", post(control::api_btw))
        .route("/api/sessions/{id}/tasks", get(control::api_tasks))
        .route(
            "/api/sessions/{id}/tasks/{tid}/kill",
            post(control::api_task_kill),
        )
        .route("/api/sessions/{id}/export", get(control::api_export))
        .route("/api/hooks", get(control::api_hooks))
        .route("/api/workflows", get(control::api_workflows))
        .route("/api/agents", get(control::api_agents))
        .route("/api/sessions/{id}/events", get(events::api_events))
        .route("/api/sessions/{id}/queue", get(queue::api_queue))
        .route(
            "/api/sessions/{id}/queue/{qid}",
            patch(queue::api_queue_patch).delete(queue::api_queue_delete),
        )
        .route(
            "/api/sessions/{id}/queue/{qid}/send",
            post(queue::api_queue_send),
        )
        .route(
            "/api/sessions/{id}/permissions/{req}",
            post(perm::api_permission),
        )
        .route(
            "/api/sessions/{id}/questions/{req}",
            post(question::api_question),
        )
        .route("/api/ask", post(question::api_ask_create))
        .route("/api/ask/{req}", get(question::api_ask_wait))
        .layer(DefaultBodyLimit::max(upload_max.saturating_add(64 * 1024)))
}
