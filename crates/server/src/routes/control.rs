use super::{map_agent_err, occupancy, session_busy};
use crate::http::{json_ok, valid_id};
use crate::service::AppState;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use ggok_core::occupy::SessionOp;
use ggok_core::parse::blocks_to_markdown;
use ggok_core::types::PromptFile;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub(crate) struct RewindBody {
    pub prompt_index: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RetryBody {
    pub prompt_index: u64,
    pub text: Option<String>,
    #[serde(default)]
    pub files: Vec<PromptFile>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ForkBody {
    pub directive: Option<String>,
    #[serde(default)]
    pub worktree: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompactBody {
    pub context: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModeBody {
    pub mode: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BtwBody {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExportQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CwdQuery {
    pub cwd: Option<String>,
}

#[derive(Clone, Copy)]
enum GateFail {
    BadId,
    NotFound,
    Busy,
}

fn gate_response(fail: GateFail) -> Response {
    match fail {
        GateFail::BadId => (StatusCode::BAD_REQUEST, "invalid session id").into_response(),
        GateFail::NotFound => (StatusCode::NOT_FOUND, "session not found").into_response(),
        GateFail::Busy => session_busy(),
    }
}

fn session_cwd(state: &AppState, id: &str) -> Result<String, GateFail> {
    state.session(id).map(|m| m.cwd).ok_or(GateFail::NotFound)
}

async fn require_control(state: &AppState, id: &str) -> Result<String, GateFail> {
    if !valid_id(id) {
        return Err(GateFail::BadId);
    }
    let cwd = session_cwd(state, id)?;
    let occ = occupancy(state, id).await;
    if ggok_core::occupy::conflict_busy(occ, SessionOp::Control) {
        return Err(GateFail::Busy);
    }
    Ok(cwd)
}

pub(crate) async fn api_rewind_points(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.rewind_points(&id, &cwd).await {
        Ok(points) => json_ok(&json!({ "points": points })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_rewind(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<RewindBody>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.rewind_execute(&id, &cwd, body.prompt_index).await {
        Ok(prompt_index) => json_ok(&json!({ "ok": true, "prompt_index": prompt_index })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_retry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<RetryBody>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state
        .agent
        .retry(&id, &cwd, body.prompt_index, body.text, body.files)
        .await
    {
        Ok(out) => json_ok(&out),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_fork(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ForkBody>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state
        .agent
        .fork_session(&id, &cwd, body.directive.as_deref(), body.worktree)
        .await
    {
        Ok(forked) => {
            super::session::insert_stub(&state, &forked.id, &forked.cwd, "");
            json_ok(&forked)
        }
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_compact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<CompactBody>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state
        .agent
        .compact_session(&id, &cwd, body.context.as_deref())
        .await
    {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_mode(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ModeBody>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.set_session_mode(&id, &cwd, &body.mode).await {
        Ok(mode) => json_ok(&json!({ "mode": mode })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let cwd = match session_cwd(&state, &id) {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.session_plan(&id, &cwd).await {
        Ok(plan) => json_ok(&plan),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_btw(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<BtwBody>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.btw(&id, &cwd, &body.text).await {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_tasks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.list_tasks(&id, &cwd).await {
        Ok(tasks) => json_ok(&json!({ "tasks": tasks })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_task_kill(
    State(state): State<Arc<AppState>>,
    Path((id, tid)): Path<(String, String)>,
) -> Response {
    let cwd = match require_control(&state, &id).await {
        Ok(c) => c,
        Err(f) => return gate_response(f),
    };
    match state.agent.kill_task(&id, &cwd, &tid).await {
        Ok(()) => json_ok(&json!({ "ok": true })),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_export(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let Some(meta) = state.session(&id) else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    };
    let format = q.format.unwrap_or_else(|| "md".into());
    if format != "md" && format != "markdown" {
        return (StatusCode::BAD_REQUEST, "unsupported export format").into_response();
    }
    let Ok(parsed) = state.parsed(&meta) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "parse failed").into_response();
    };
    let md = blocks_to_markdown(&parsed.blocks);
    let filename = format!("{id}.md");
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/markdown; charset=utf-8")
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(md))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub(crate) async fn api_hooks(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CwdQuery>,
) -> Response {
    let cwd = match ext_cwd(q.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    match state.agent.list_hooks(&cwd).await {
        Ok(v) => json_ok(&v),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_workflows(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CwdQuery>,
) -> Response {
    let cwd = match ext_cwd(q.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    match state.agent.list_workflows(&cwd).await {
        Ok(v) => json_ok(&v),
        Err(e) => map_agent_err(&e),
    }
}

pub(crate) async fn api_agents(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CwdQuery>,
) -> Response {
    let cwd = match ext_cwd(q.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    json_ok(&json!({ "agents": list_agent_dirs(&cwd) }))
}

fn ext_cwd(raw: Option<&str>, roots: &[PathBuf]) -> Result<PathBuf, String> {
    super::ext::mcp_cwd(raw, roots)
}

fn list_agent_dirs(cwd: &FsPath) -> Vec<Value> {
    let mut out = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        cwd.join(".grok/agents"),
        cwd.join(".grok/personas"),
        FsPath::new(&home).join(".grok/agents"),
        FsPath::new(&home).join(".grok/personas"),
    ];
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in entries.flatten() {
            let path = ent.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            out.push(json!({
                "name": name,
                "path": path.display().to_string(),
                "kind": if dir.ends_with("personas") { "persona" } else { "agent" }
            }));
        }
    }
    out
}
