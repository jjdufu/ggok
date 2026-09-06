use ggok_server::version_view;

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
