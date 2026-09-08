use ggok_core::scan::scan;
use std::fs;

fn write_summary(dir: &std::path::Path, id: &str, cwd: &str, title: &str, n: u64) {
    fs::create_dir_all(dir).expect("mkdir");
    fs::write(
        dir.join("summary.json"),
        format!(
            r#"{{
            "info": {{"id": "{id}", "cwd": "{cwd}"}},
            "generated_title": "{title}",
            "current_model_id": "grok-4.6",
            "num_messages": {n},
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:01Z"
        }}"#
        ),
    )
    .expect("summary");
    if n > 0 {
        fs::write(dir.join("updates.jsonl"), "{}\n").expect("jsonl");
    }
}

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
    assert!(meta.subagent_of.is_none());
}

#[test]
fn scan_marks_subagent_children_and_hides_them_from_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let proj = dir.path().join("sessions").join("%2Ftmp%2Fproj");
    let parent_id = "550e8400-e29b-41d4-a716-446655440000";
    let child_id = "550e8400-e29b-41d4-a716-446655440001";
    let parent = proj.join(parent_id);
    let child = proj.join(child_id);
    write_summary(&parent, parent_id, "/tmp/proj", "Parent chat", 2);
    write_summary(&child, child_id, "/tmp/proj", "Writer subagent", 4);
    let meta_dir = parent.join("subagents").join(child_id);
    fs::create_dir_all(&meta_dir).expect("subagents");
    fs::write(
        meta_dir.join("meta.json"),
        format!(
            r#"{{
            "subagent_id": "{child_id}",
            "parent_session_id": "{parent_id}",
            "child_session_id": "{child_id}",
            "subagent_type": "general-purpose",
            "description": "[writer] Write design doc"
        }}"#
        ),
    )
    .expect("meta");

    let index = scan(dir.path()).expect("scan");
    let parent_meta = index.get(parent_id).expect("parent");
    assert!(parent_meta.subagent_of.is_none());
    assert!(parent_meta.parent_id.is_none());
    let child_meta = index.get(child_id).expect("child still indexed");
    assert_eq!(child_meta.subagent_of.as_deref(), Some(parent_id));
    assert!(child_meta.parent_id.is_none());

    let rows = index.list(None, None, false, None);
    let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&parent_id), "{ids:?}");
    assert!(!ids.contains(&child_id), "{ids:?}");

    let projects = index.projects();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].sessions, 1);
}

#[test]
fn scan_keeps_summary_parent_id_as_fork_not_subagent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let proj = dir.path().join("sessions").join("%2Ftmp%2Fproj");
    let parent_id = "550e8400-e29b-41d4-a716-446655440010";
    let fork_id = "550e8400-e29b-41d4-a716-446655440011";
    write_summary(&proj.join(parent_id), parent_id, "/tmp/proj", "Root", 2);
    let fork = proj.join(fork_id);
    fs::create_dir_all(&fork).expect("mkdir");
    fs::write(
        fork.join("summary.json"),
        format!(
            r#"{{
            "info": {{"id": "{fork_id}", "cwd": "/tmp/proj"}},
            "generated_title": "Forked chat",
            "parent_session_id": "{parent_id}",
            "num_messages": 2,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:02Z"
        }}"#
        ),
    )
    .expect("summary");
    fs::write(fork.join("updates.jsonl"), "{}\n").expect("jsonl");

    let index = scan(dir.path()).expect("scan");
    let fork_meta = index.get(fork_id).expect("fork");
    assert_eq!(fork_meta.parent_id.as_deref(), Some(parent_id));
    assert!(fork_meta.subagent_of.is_none());
    let rows = index.list(None, None, false, None);
    assert!(rows.iter().any(|r| r.id == fork_id));
}
