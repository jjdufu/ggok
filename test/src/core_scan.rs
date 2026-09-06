use ggok_core::scan::scan;
use std::fs;

#[test]
fn scan_reads_reasoning_effort_from_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sess = dir
        .path()
        .join("sessions")
        .join("%2Ftmp%2Fproj")
        .join("550e8400-e29b-41d4-a716-446655440000");
    fs::create_dir_all(&sess).expect("mkdir");
    fs::write(
        sess.join("summary.json"),
        r#"{
            "info": {"id": "550e8400-e29b-41d4-a716-446655440000", "cwd": "/tmp/proj"},
            "generated_title": "TUI session",
            "current_model_id": "grok-4.6",
            "reasoning_effort": "xhigh",
            "num_messages": 2,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:01Z"
        }"#,
    )
    .expect("summary");
    fs::write(sess.join("updates.jsonl"), "{}\n").expect("jsonl");
    let index = scan(dir.path()).expect("scan");
    let meta = index
        .get("550e8400-e29b-41d4-a716-446655440000")
        .expect("session");
    assert_eq!(meta.model, "grok-4.6");
    assert_eq!(meta.effort, "xhigh");
    assert_eq!(meta.title, "TUI session");
}
