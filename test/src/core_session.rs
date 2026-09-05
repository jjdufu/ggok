use ggok_core::session::{is_pinned, load_pins, pins_path_from_agent_pid, rename_summary, set_pinned};
use std::fs;
use std::path::Path;

#[test]
fn pins_path_next_to_agent_pid() {
    let p = pins_path_from_agent_pid(Path::new("/var/ggok/grok-agent.pid"));
    assert_eq!(p, Path::new("/var/ggok/pins.json"));
    assert_eq!(
        pins_path_from_agent_pid(Path::new("grok-agent.pid")),
        Path::new("pins.json")
    );
}

#[test]
fn pin_unpin_and_reject_non_uuid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pins.json");
    assert!(load_pins(&path).is_empty());
    assert!(!is_pinned(&path, "11111111-1111-1111-1111-111111111111"));

    let id = "550e8400-e29b-41d4-a716-446655440000";
    let ids = set_pinned(&path, id, true).expect("pin");
    assert_eq!(ids, vec![id]);
    assert!(is_pinned(&path, id));

    let id2 = "550e8400-e29b-41d4-a716-446655440001";
    let ids = set_pinned(&path, id2, true).expect("pin 2");
    assert_eq!(ids[0], id2);

    let ids = set_pinned(&path, id2, false).expect("unpin");
    assert_eq!(ids, vec![id]);

    assert!(set_pinned(&path, "not-a-uuid", true).is_err());
}

#[test]
fn load_pins_drops_invalid_ids() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pins.json");
    fs::write(&path, r#"{"ids":["nope","550e8400-e29b-41d4-a716-446655440000"]}"#).expect("write");
    let ids = load_pins(&path);
    assert_eq!(ids.len(), 1);
}

#[test]
fn rename_summary_validates_and_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(rename_summary(dir.path(), "id", "/cwd", "  ").is_err());
    assert!(rename_summary(dir.path(), "id", "/cwd", "bad\nline").is_err());
    let too_long = "x".repeat(201);
    assert!(rename_summary(dir.path(), "id", "/cwd", &too_long).is_err());

    let title = rename_summary(dir.path(), "sid", "/tmp/proj", "  Hello  ").expect("rename");
    assert_eq!(title, "Hello");
    let raw = fs::read_to_string(dir.path().join("summary.json")).expect("read");
    assert!(raw.contains("Hello"));
    assert!(raw.contains("title_is_manual"));
}
