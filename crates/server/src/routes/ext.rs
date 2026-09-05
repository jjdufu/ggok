use crate::http::{json_ok, public_err};
use crate::service::AppState;
use axum::Json;
use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ggok_agent::ext::{mcp, plugin, skill};
use ggok_core::paths;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub(crate) struct McpQuery {
    pub cwd: Option<String>,
}

pub(crate) fn mcp_cwd(raw: Option<&str>, roots: &[PathBuf]) -> Result<PathBuf, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(cwd) => paths::cwd_allowed(cwd, roots).map_err(|e| e.to_string()),
        None => Ok(std::env::var("HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map_or_else(|| PathBuf::from("/"), PathBuf::from)),
    }
}

pub(crate) async fn api_mcp_get(
    State(state): State<Arc<AppState>>,
    Query(q): Query<McpQuery>,
) -> Response {
    let cwd = match mcp_cwd(q.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    match mcp::snapshot(&state.grok_bin, &cwd).await {
        Ok(v) => json_ok(&v),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpPost {
    pub op: String,
    pub cwd: Option<String>,
    pub name: Option<String>,
    pub transport: Option<String>,
    pub command_or_url: Option<String>,
    pub args: Option<Vec<String>>,
    pub scope: Option<String>,
    pub env: Option<Vec<String>>,
    pub headers: Option<Vec<String>>,
}

pub(crate) async fn api_mcp_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<McpPost>,
) -> Response {
    let cwd = match mcp_cwd(body.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    let name = body.name.as_deref().unwrap_or("").trim();
    let out = match body.op.as_str() {
        "add" => {
            let cmd = body.command_or_url.as_deref().unwrap_or("").trim();
            if cmd.is_empty() {
                return (StatusCode::BAD_REQUEST, "command_or_url required").into_response();
            }
            mcp::add(
                &state.grok_bin,
                &cwd,
                mcp::AddSpec {
                    name,
                    transport: body.transport.as_deref().unwrap_or("stdio"),
                    command_or_url: cmd,
                    args: body.args.as_deref().unwrap_or(&[]),
                    scope: body.scope.as_deref().unwrap_or("user"),
                    env: body.env.as_deref().unwrap_or(&[]),
                    headers: body.headers.as_deref().unwrap_or(&[]),
                },
            )
            .await
        }
        "remove" => {
            mcp::remove(
                &state.grok_bin,
                &cwd,
                name,
                body.scope
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
            )
            .await
        }
        "enable" => mcp::enable(&state.grok_bin, &cwd, name).await,
        "disable" => mcp::disable(&state.grok_bin, &cwd, name).await,
        other => return (StatusCode::BAD_REQUEST, format!("unknown op {other}")).into_response(),
    };
    match out {
        Ok(text) => json_ok(&serde_json::json!({ "ok": true, "text": text })),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PluginPost {
    pub op: String,
    pub cwd: Option<String>,
    pub name: Option<String>,
    pub source: Option<String>,
}

pub(crate) async fn api_plugins_get(
    State(state): State<Arc<AppState>>,
    Query(q): Query<McpQuery>,
) -> Response {
    let cwd = match mcp_cwd(q.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    match plugin::snapshot(&state.grok_bin, &cwd).await {
        Ok(v) => json_ok(&v),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}

pub(crate) async fn api_plugins_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PluginPost>,
) -> Response {
    let cwd = match mcp_cwd(body.cwd.as_deref(), &state.workspace_roots) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    let name = body.name.as_deref().unwrap_or("").trim();
    let source = body.source.as_deref().unwrap_or("").trim();
    let out = match body.op.as_str() {
        "install" => plugin::install(&state.grok_bin, &cwd, source).await,
        "uninstall" => plugin::uninstall(&state.grok_bin, &cwd, name).await,
        "enable" => plugin::enable(&state.grok_bin, &cwd, name).await,
        "disable" => plugin::disable(&state.grok_bin, &cwd, name).await,
        "update" => {
            plugin::update(
                &state.grok_bin,
                &cwd,
                if name.is_empty() { None } else { Some(name) },
            )
            .await
        }
        "marketplace_add" => plugin::marketplace_add(&state.grok_bin, &cwd, source).await,
        "marketplace_remove" => plugin::marketplace_remove(&state.grok_bin, &cwd, source).await,
        "marketplace_update" => {
            plugin::marketplace_update(
                &state.grok_bin,
                &cwd,
                if source.is_empty() {
                    None
                } else {
                    Some(source)
                },
            )
            .await
        }
        other => return (StatusCode::BAD_REQUEST, format!("unknown op {other}")).into_response(),
    };
    match out {
        Ok(text) => json_ok(&serde_json::json!({ "ok": true, "text": text })),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SkillPost {
    pub op: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub body: Option<String>,
}

pub(crate) async fn api_skills_get(
    State(state): State<Arc<AppState>>,
    Query(q): Query<McpQuery>,
) -> Response {
    let cwd = match q.cwd.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => match paths::cwd_allowed(raw, &state.workspace_roots) {
            Ok(p) => Some(p),
            Err(e) => return (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
        },
        None => None,
    };
    json_ok(&skill::list(&state.grok_home, cwd.as_deref()))
}

#[derive(Debug, Deserialize)]
pub(crate) struct SkillItemQuery {
    pub name: String,
    pub scope: Option<String>,
    pub cwd: Option<String>,
}

pub(crate) async fn api_skills_item(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SkillItemQuery>,
) -> Response {
    let cwd = match q.cwd.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => match paths::cwd_allowed(raw, &state.workspace_roots) {
            Ok(p) => Some(p),
            Err(e) => return (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
        },
        None => None,
    };
    match skill::detail(
        &state.grok_home,
        cwd.as_deref(),
        &q.name,
        q.scope.as_deref(),
    ) {
        Ok(v) => json_ok(&v),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}

pub(crate) async fn api_skills_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SkillPost>,
) -> Response {
    if body.op != "create" {
        return (StatusCode::BAD_REQUEST, format!("unknown op {}", body.op)).into_response();
    }
    match skill::create(
        &state.grok_home,
        body.name.as_deref().unwrap_or(""),
        body.description.as_deref().unwrap_or(""),
        body.body.as_deref().unwrap_or(""),
    ) {
        Ok(v) => json_ok(&v),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}

pub(crate) async fn api_skills_upload(
    State(state): State<Arc<AppState>>,
    mut form: Multipart,
) -> Response {
    let mut uploaded_name = None;
    let mut bytes = None;
    while let Ok(Some(field)) = form.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            uploaded_name = field.file_name().map(ToOwned::to_owned);
            bytes = field.bytes().await.ok().map(|b| b.to_vec());
        }
    }
    let Some(filename) = uploaded_name else {
        return (StatusCode::BAD_REQUEST, "file required").into_response();
    };
    let Some(bytes) = bytes else {
        return (StatusCode::BAD_REQUEST, "file required").into_response();
    };
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > state.upload_max_bytes {
        return (StatusCode::BAD_REQUEST, "file too large").into_response();
    }
    match skill::upload(&state.grok_home, &filename, &bytes) {
        Ok(v) => json_ok(&v),
        Err(e) => (StatusCode::BAD_REQUEST, public_err(&e.to_string())).into_response(),
    }
}
