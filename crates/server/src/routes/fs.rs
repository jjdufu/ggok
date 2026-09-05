use crate::http::json_ok;
use crate::service::AppState;
use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ggok_core::paths;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub(crate) struct DirsQuery {
    parent: Option<String>,
}

pub(crate) async fn api_dirs(State(state): State<Arc<AppState>>, Query(q): Query<DirsQuery>) -> Response {
    match paths::list_dirs(q.parent.as_deref(), &state.workspace_roots) {
        Ok(rows) => json_ok(&rows),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct FsQuery {
    cwd: String,
    q: Option<String>,
}

pub(crate) async fn api_fs(State(state): State<Arc<AppState>>, Query(q): Query<FsQuery>) -> Response {
    if let Err(e) = paths::cwd_allowed(&q.cwd, &state.workspace_roots) {
        return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    match paths::fs_complete(&q.cwd, q.q.as_deref().unwrap_or("")) {
        Ok(rows) => json_ok(&rows),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct UploadGetQuery {
    path: String,
}

pub(crate) async fn api_upload_get(Query(q): Query<UploadGetQuery>) -> Response {
    let Ok(path) = paths::open_upload(&q.path) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    match std::fs::read(&path) {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .essence_str()
                .to_string();
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .replace('"', "");
            let mut headers = axum::http::HeaderMap::new();
            let mime_hv = axum::http::HeaderValue::from_str(&mime).unwrap_or_else(|_| {
                axum::http::HeaderValue::from_static("application/octet-stream")
            });
            headers.insert(axum::http::header::CONTENT_TYPE, mime_hv);
            let ascii: String = name.chars().filter(|c| c.is_ascii() && *c != '"').collect();
            if !ascii.is_empty() {
                let disp = format!("inline; filename=\"{ascii}\"");
                if let Ok(v) = axum::http::HeaderValue::from_str(&disp) {
                    headers.insert(axum::http::header::CONTENT_DISPOSITION, v);
                }
            }
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

pub(crate) async fn api_uploads(State(state): State<Arc<AppState>>, mut form: Multipart) -> Response {
    let mut cwd = None;
    let mut uploaded_name = None;
    let mut bytes = None;
    while let Ok(Some(field)) = form.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "cwd" {
            cwd = field.text().await.ok();
        } else if name == "file" {
            uploaded_name = field.file_name().map(ToOwned::to_owned);
            bytes = field.bytes().await.ok().map(|b| b.to_vec());
        }
    }
    let Some(cwd_raw) = cwd else {
        return (StatusCode::BAD_REQUEST, "cwd required").into_response();
    };
    let Some(filename) = uploaded_name else {
        return (StatusCode::BAD_REQUEST, "file required").into_response();
    };
    let Some(bytes) = bytes else {
        return (StatusCode::BAD_REQUEST, "file required").into_response();
    };
    if let Err(e) = paths::cwd_allowed(&cwd_raw, &state.workspace_roots) {
        return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    let bytes = match tokio::task::spawn_blocking({
        let filename = filename.clone();
        move || paths::compress_upload(&filename, bytes)
    })
    .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("upload compress join: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "compress failed").into_response();
        }
    };
    match paths::save_upload(&filename, &bytes, state.upload_max_bytes) {
        Ok(path) => {
            let mime = mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .essence_str()
                .to_string();
            let abs = path.to_string_lossy().into_owned();
            json_ok(&json!({
                "path": abs,
                "name": filename,
                "mime": mime,
                "rel": abs,
            }))
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
