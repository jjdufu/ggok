use crate::http::{json_ok, public_err};
use crate::service::AppState;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ggok_core::workspace;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct WsQuery {
    cwd: String,
    path: Option<String>,
}

fn rel_of(q: &WsQuery) -> &str {
    q.path.as_deref().unwrap_or("")
}

fn status_for(msg: &str) -> StatusCode {
    let m = msg.to_ascii_lowercase();
    if m.contains("not found") {
        StatusCode::NOT_FOUND
    } else if m.contains("too large") || m.contains("too many") {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::BAD_REQUEST
    }
}

fn ws_err(err: &anyhow::Error) -> Response {
    let msg = public_err(&err.to_string());
    (status_for(&msg), msg).into_response()
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['\r', '\n', '"'], "_")
}

fn content_disposition(filename: &str) -> String {
    let cleaned = sanitize_filename(filename);
    let ascii: String = cleaned
        .chars()
        .filter(|c| c.is_ascii() && *c != '"')
        .collect();
    let ascii = if ascii.is_empty() {
        "file"
    } else {
        ascii.as_str()
    };
    if cleaned.is_ascii() {
        format!("attachment; filename=\"{ascii}\"")
    } else {
        let encoded = utf8_percent_encode(&cleaned, NON_ALPHANUMERIC);
        format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
    }
}

fn attachment_headers(mime: &str, filename: &str, len: Option<u64>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let mime_hv = HeaderValue::from_str(mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    headers.insert(CONTENT_TYPE, mime_hv);
    if let Ok(v) = HeaderValue::from_str(&content_disposition(filename)) {
        headers.insert(CONTENT_DISPOSITION, v);
    }
    if let Some(n) = len
        && let Ok(v) = HeaderValue::from_str(&n.to_string())
    {
        headers.insert(CONTENT_LENGTH, v);
    }
    headers
}

fn download_basename(path: &Path, zip: bool) -> String {
    let base = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(sanitize_filename)
        .unwrap_or_default();
    if zip {
        if base.is_empty() {
            "workspace.zip".to_string()
        } else if Path::new(&base)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            base
        } else {
            format!("{base}.zip")
        }
    } else if base.is_empty() {
        "file".to_string()
    } else {
        base
    }
}

fn stream_path(path: PathBuf, delete_after: bool) -> Body {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(8);
    tokio::spawn(async move {
        match tokio::fs::File::open(&path).await {
            Ok(mut file) => {
                let mut buf = vec![0_u8; 64 * 1024];
                loop {
                    match file.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if tx.send(Ok(buf[..n].to_vec())).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e)).await;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e)).await;
            }
        }
        drop(tx);
        if delete_after {
            let _ = tokio::fs::remove_file(&path).await;
        }
    });
    Body::from_stream(ReceiverStream::new(rx))
}

pub(crate) async fn api_workspace_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsQuery>,
) -> Response {
    match workspace::list_workspace(&q.cwd, rel_of(&q), &state.workspace_roots) {
        Ok(list) => json_ok(&list),
        Err(e) => ws_err(&e),
    }
}

pub(crate) async fn api_workspace_delete(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsQuery>,
) -> Response {
    match workspace::delete_workspace(&q.cwd, rel_of(&q), &state.workspace_roots) {
        Ok(()) => json_ok(&json!({"ok": true})),
        Err(e) => ws_err(&e),
    }
}

pub(crate) async fn api_workspace_file(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsQuery>,
) -> Response {
    match workspace::open_workspace_file(&q.cwd, rel_of(&q), &state.workspace_roots) {
        Ok((path, size)) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .essence_str()
                .to_string();
            let name = download_basename(&path, false);
            let headers = attachment_headers(&mime, &name, Some(size));
            (StatusCode::OK, headers, stream_path(path, false)).into_response()
        }
        Err(e) => ws_err(&e),
    }
}

pub(crate) async fn api_workspace_archive(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsQuery>,
) -> Response {
    let rel = rel_of(&q).to_string();
    let cwd = q.cwd.clone();
    let roots = state.workspace_roots.clone();
    let tmp = std::env::temp_dir().join(format!("ggok-ws-{}.zip", Uuid::new_v4()));
    let tmp_write = tmp.clone();
    let join = tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::create(&tmp_write)?;
        workspace::write_archive(&cwd, &rel, &roots, &mut file)?;
        file.sync_all()?;
        Ok::<(), anyhow::Error>(())
    })
    .await;
    match join {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return ws_err(&e);
        }
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            tracing::warn!("workspace archive join: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "archive failed").into_response();
        }
    }
    let size = match tokio::fs::metadata(&tmp).await {
        Ok(m) => m.len(),
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                public_err(&e.to_string()),
            )
                .into_response();
        }
    };
    let name = if rel_of(&q).trim().is_empty() {
        download_basename(Path::new(&q.cwd), true)
    } else {
        download_basename(Path::new(rel_of(&q)), true)
    };
    let headers = attachment_headers("application/zip", &name, Some(size));
    (StatusCode::OK, headers, stream_path(tmp, true)).into_response()
}
