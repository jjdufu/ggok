use ggok_core::prefs::{
    LastModel, last_model_path, load_last_model, merge_grok_model_defaults, merge_models_section,
    resolve_choice, save_last_model,
};
use std::fs;

#[test]
fn last_model_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = last_model_path(dir.path());
    assert_eq!(load_last_model(&path), LastModel::default());
    let last = LastModel {
        model: "grok-4.6".into(),
        effort: "xhigh".into(),
    };
    save_last_model(&path, &last).expect("save");
    assert_eq!(load_last_model(&path), last);
}

#[test]
fn resolve_choice_prefers_explicit_then_last() {
    let last = LastModel {
        model: "grok-4.6".into(),
        effort: "xhigh".into(),
    };
    assert_eq!(
        resolve_choice(Some("other"), Some("high"), &last),
        (Some("other".into()), Some("high".into()))
    );
    assert_eq!(
        resolve_choice(None, None, &last),
        (Some("grok-4.6".into()), Some("xhigh".into()))
    );
    assert_eq!(
        resolve_choice(Some(""), Some("  "), &LastModel::default()),
        (None, None)
    );
}

#[test]
fn merge_models_section_appends_when_missing() {
    let out = merge_models_section("[cli]\nauto_update = true\n", "grok-4.6", "xhigh");
    assert!(out.contains("[cli]"));
    assert!(out.contains("auto_update = true"));
    assert!(out.contains("[models]"));
    assert!(out.contains("default = \"grok-4.6\""));
    assert!(out.contains("default_reasoning_effort = \"xhigh\""));
}

#[test]
fn merge_models_section_updates_existing_keys_only() {
    let src = "[ui]\nyolo = false\n\n[models]\ndefault = \"grok-4.5\"\ndefault_reasoning_effort = \"high\"\nextra = 1\n\n[plugins]\nenabled = [\"x\"]\n";
    let out = merge_models_section(src, "grok-4.6", "xhigh");
    assert!(out.contains("[ui]"));
    assert!(out.contains("yolo = false"));
    assert!(out.contains("default = \"grok-4.6\""));
    assert!(out.contains("default_reasoning_effort = \"xhigh\""));
    assert!(out.contains("extra = 1"));
    assert!(out.contains("[plugins]"));
    assert!(!out.contains("grok-4.5"));
}

#[test]
fn merge_models_section_skips_nested_model_tables() {
    let src = "[model.grok-4.6]\nname = \"Grok\"\n\n[models]\ndefault = \"grok-4.6\"\n";
    let out = merge_models_section(src, "grok-4.6", "xhigh");
    assert!(out.contains("[model.grok-4.6]"));
    assert!(out.contains("name = \"Grok\""));
    assert!(out.contains("default_reasoning_effort = \"xhigh\""));
}

#[test]
fn merge_grok_model_defaults_writes_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    fs::write(&path, "[cli]\ninstaller = \"internal\"\n").expect("write");
    merge_grok_model_defaults(&path, "grok-4.6", "xhigh").expect("merge");
    let raw = fs::read_to_string(&path).expect("read");
    assert!(raw.contains("[cli]"));
    assert!(raw.contains("default_reasoning_effort = \"xhigh\""));
}
