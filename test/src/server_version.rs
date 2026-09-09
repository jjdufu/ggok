use ggok_server::version_view;
use std::fs;
use std::path::PathBuf;

fn server_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("crates")
        .join("server")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn version_view_unknown_latest_is_null() {
    let view = version_view("0.1.5", None);
    let json = serde_json::to_value(&view).expect("json");
    assert_eq!(json["version"], "0.1.5");
    assert!(json["latest"].is_null(), "{json}");
    assert_eq!(json["update_available"], false);
}

#[test]
fn version_view_same_is_not_an_update() {
    let view = version_view("0.1.5", Some("0.1.5"));
    let json = serde_json::to_value(&view).expect("json");
    assert_eq!(json["latest"], "0.1.5");
    assert!(!view.update_available);
}

#[test]
fn version_view_newer_sets_flag() {
    let view = version_view("0.1.5", Some("0.1.6"));
    assert_eq!(view.latest.as_deref(), Some("0.1.6"));
    assert!(view.update_available);
}

#[test]
fn version_look_waits_when_cache_is_stale() {
    let src = server_file("src/release.rs");
    assert!(
        src.contains("const CACHE_OK: Duration = Duration::from_secs(60)")
            && !src.contains("Duration::from_hours(6)"),
        "version cache must match the 60s account look, not 6h:\n{src}"
    );
    assert!(
        src.contains("pub async fn snapshot()")
            && src.contains("Hit::StaleOk | Hit::Miss => load_shared().await"),
        "a stale look must wait for GitHub instead of returning the old tag:\n{src}"
    );
}

#[test]
fn api_version_awaits_snapshot() {
    let src = server_file("src/routes/meta.rs");
    assert!(
        src.contains("crate::release::snapshot().await"),
        "/api/version must wait for the look-path snapshot:\n{src}"
    );
}
