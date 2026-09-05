use crate::auth::{
    append_set_cookie, bearer_token, build_session_cookie, clear_session_cookie, constant_time_eq,
    cookie_from_header, now_unix, sign_cookie, too_many, unauthorized, verify_cookie,
};
use crate::service::AppState;
use crate::static_files;
use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::extract::{ConnectInfo, State};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS,
};
use axum::http::{HeaderName, HeaderValue, Method, Request, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use uuid::Uuid;

const REFERRER_POLICY: HeaderName = HeaderName::from_static("referrer-policy");

pub(crate) fn router(state: Arc<AppState>) -> Router {
    let upload_max = usize::try_from(state.upload_max_bytes).unwrap_or(20 * 1024 * 1024);
    Router::new()
        .route("/login", get(login_page).post(login))
        .route("/logout", post(logout))
        .route("/", get(index_page))
        .route("/app.css", get(asset_app_css))
        .route("/app.js", get(asset_app_js))
        .route("/i18n.js", get(asset_i18n_js))
        .route("/api/projects", get(api_projects))
        .merge(crate::routes::router(upload_max))
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(upload_max.saturating_add(64 * 1024)))
        .layer(middleware::from_fn_with_state(state.clone(), auth_gate))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(CACHE_CONTROL, cache_control_for(&path));
    res
}

fn cache_control_for(path: &str) -> HeaderValue {
    if path.starts_with("/fonts/")
        || path == "/app.css"
        || path == "/app.js"
        || path == "/i18n.js"
    {
        HeaderValue::from_static("public, max-age=31536000, immutable")
    } else {
        HeaderValue::from_static("no-store")
    }
}

fn is_public(method: &Method, path: &str) -> bool {
    matches!(
        (method, path),
        (&Method::GET | &Method::POST, "/login")
            | (
                &Method::GET,
                "/app.css" | "/app.js" | "/i18n.js" | "/favicon.svg",
            )
    ) || (*method == Method::GET && path.starts_with("/fonts/"))
}

fn is_authorized(state: &AppState, req: &Request<Body>) -> bool {
    if let Some(token) = bearer_token(
        req.headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok()),
    ) && constant_time_eq(token, &state.token)
    {
        return true;
    }
    let cookie = cookie_from_header(
        req.headers().get(COOKIE).and_then(|v| v.to_str().ok()),
        &state.cookie_name,
    );
    cookie.is_some_and(|c| verify_cookie(&state.cookie_key, &c, now_unix()))
}

async fn auth_gate(State(state): State<Arc<AppState>>, req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if is_public(&method, &path) {
        return next.run(req).await;
    }
    if is_authorized(&state, &req) {
        return next.run(req).await;
    }
    if method == Method::GET && !path.starts_with("/api/") {
        return Response::builder()
            .status(StatusCode::FOUND)
            .header(LOCATION, "/login")
            .body(Body::empty())
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    let (status, msg) = unauthorized();
    (status, msg).into_response()
}

async fn login_page() -> Response {
    embed_html("login.html")
}

async fn index_page() -> Response {
    embed_html("index.html")
}

async fn asset_app_css() -> Response {
    embed_asset("app.css", "text/css; charset=utf-8")
}

async fn asset_app_js() -> Response {
    embed_asset("app.js", "application/javascript; charset=utf-8")
}

async fn asset_i18n_js() -> Response {
    embed_asset("i18n.js", "application/javascript; charset=utf-8")
}

fn asset_bust() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let mut hasher = Sha256::new();
        for name in ["app.css", "app.js", "i18n.js"] {
            if let Some((data, _)) = static_files::get(name) {
                hasher.update(&*data);
            }
        }
        let hex = hex::encode(hasher.finalize());
        hex[..12].to_string()
    })
}

fn bust_html(html: &str) -> String {
    let v = asset_bust();
    html.replace("href=\"/app.css\"", &format!("href=\"/app.css?v={v}\""))
        .replace("src=\"/app.js\"", &format!("src=\"/app.js?v={v}\""))
        .replace("src=\"/i18n.js\"", &format!("src=\"/i18n.js?v={v}\""))
}

fn embed_html(name: &str) -> Response {
    match static_files::get(name) {
        Some((data, _)) => match String::from_utf8(data.into_owned()) {
            Ok(html) => html_ok(bust_html(&html).into_bytes()),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "html not utf-8").into_response(),
        },
        None => (StatusCode::NOT_FOUND, "missing asset").into_response(),
    }
}

fn embed_asset(name: &str, mime: &'static str) -> Response {
    match static_files::get(name) {
        Some((data, _)) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, mime)
            .body(Body::from(data.into_owned()))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        None => (StatusCode::NOT_FOUND, "missing asset").into_response(),
    }
}

fn html_ok(bytes: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    token: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Form(form): Form<LoginForm>,
) -> Response {
    let ip = addr.ip();
    let now = Instant::now();
    {
        let mut limiter = state.login_fails.lock();
        if limiter.blocked(ip, now) {
            let (status, msg) = too_many();
            return (status, msg).into_response();
        }
    }
    if !constant_time_eq(form.token.trim(), &state.token) {
        state.login_fails.lock().record_fail(ip, now);
        let (status, msg) = unauthorized();
        return (status, msg).into_response();
    }
    state.login_fails.lock().clear(ip);
    let value = sign_cookie(&state.cookie_key, now_unix());
    let cookie = build_session_cookie(&state.cookie_name, &value);
    let mut res = Response::builder()
        .status(StatusCode::FOUND)
        .header(LOCATION, "/")
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    append_set_cookie(res.headers_mut(), &cookie);
    res
}

async fn logout(State(state): State<Arc<AppState>>) -> Response {
    let cookie = clear_session_cookie(&state.cookie_name);
    let mut res = Response::builder()
        .status(StatusCode::FOUND)
        .header(LOCATION, "/login")
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    append_set_cookie(res.headers_mut(), &cookie);
    res
}

async fn api_projects(State(state): State<Arc<AppState>>) -> Response {
    let rows = state.sessions.read().projects();
    json_ok(&rows)
}

async fn fallback(uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if path.contains('.') {
        let name = path.trim_start_matches('/');
        if let Some((data, mime)) = static_files::get(name) {
            return Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, mime)
                .body(Body::from(data.into_owned()))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    Redirect::to("/").into_response()
}

pub(crate) fn valid_id(id: &str) -> bool {
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return false;
    }
    Uuid::parse_str(id).is_ok()
}

pub(crate) fn json_ok<T: serde::Serialize>(value: &T) -> Response {
    json_status(StatusCode::OK, value)
}

#[must_use]
pub(crate) fn public_err(raw: &str) -> String {
    let s = raw.trim();
    let line = s
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.eq_ignore_ascii_case("error") && !l.starts_with("stack backtrace"))
        .unwrap_or(s);
    let line = line.strip_prefix("Error: ").unwrap_or(line);
    let count = line.chars().count();
    if count <= 240 {
        return line.to_string();
    }
    let mut out: String = line.chars().take(240).collect();
    out.push_str("...");
    out
}

pub(crate) fn json_status<T: serde::Serialize>(status: StatusCode, value: &T) -> Response {
    match serde_json::to_vec(value) {
        Ok(bytes) => Response::builder()
            .status(status)
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response(),
    }
}
