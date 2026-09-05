pub(crate) mod events;
pub(crate) mod ext;
pub(crate) mod fs;
pub(crate) mod meta;
pub(crate) mod perm;
pub(crate) mod prompt;
pub(crate) mod queue;
pub(crate) mod session;
pub(crate) mod workspace;

use crate::service::AppState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use ggok_core::occupy::{self, Occupancy};
use std::sync::Arc;

pub(crate) async fn occupancy(state: &AppState, id: &str) -> Occupancy {
    let live = state.agent.live_view(id).await;
    let agent_pid = occupy::agent_pid(&state.agent_pid_file, state.agent.child_pid().await);
    let cli = occupy::cli_sessions(&state.grok_home, agent_pid);
    occupy::classify(id, live.as_ref(), &cli)
}

pub(crate) fn router(upload_max: usize) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/runtime", get(meta::api_runtime))
        .route("/api/dirs", get(fs::api_dirs))
        .route("/api/status", get(meta::api_status))
        .route("/api/account", get(meta::api_account))
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
        .route(
            "/api/sessions/{id}/tools/{tool_id}",
            get(session::api_tool),
        )
        .route("/api/sessions/{id}/load", post(session::api_load))
        .route("/api/sessions/{id}/prompt", post(prompt::api_prompt))
        .route("/api/sessions/{id}/cancel", post(prompt::api_cancel))
        .route("/api/sessions/{id}/model", post(session::api_model))
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
        .layer(DefaultBodyLimit::max(upload_max.saturating_add(64 * 1024)))
}
