use ggok_core::occupy::{LiveView, Source, agent_pid, classify, cli_sessions, read_pid_file};
use std::collections::HashMap;
use std::fs;

fn live(loaded: bool, running: bool) -> LiveView {
    LiveView {
        loaded,
        running,
        model: "grok".into(),
        effort: "high".into(),
    }
}

#[test]
fn classify_prefers_loaded_agent() {
    let cli = HashMap::from([("s1".into(), 1_u32)]);
    let occ = classify("s1", Some(&live(true, true)), &cli);
    assert_eq!(occ.source, Source::Agent);
    assert!(occ.writable);
    assert!(occ.running);

    let idle = classify("s1", Some(&live(true, false)), &cli);
    assert_eq!(idle.source, Source::Agent);
    assert!(!idle.running);
}

#[test]
fn classify_cli_when_agent_not_loaded() {
    let cli = HashMap::from([("s1".into(), 9_u32)]);
    let occ = classify("s1", Some(&live(false, false)), &cli);
    assert_eq!(occ.source, Source::Cli);
    assert!(occ.running);
}

#[test]
fn classify_disk_fallback() {
    let occ = classify("missing", None, &HashMap::<String, u32>::new());
    assert_eq!(occ.source, Source::Disk);
    assert!(occ.writable);
    assert!(!occ.running);
}

#[test]
fn source_as_str() {
    assert_eq!(Source::Agent.as_str(), "agent");
    assert_eq!(Source::Cli.as_str(), "cli");
    assert_eq!(Source::Disk.as_str(), "disk");
}

#[test]
fn read_pid_file_and_agent_pid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("agent.pid");
    assert!(read_pid_file(&path).is_none());
    fs::write(&path, "  4321 \n").expect("write pid");
    assert_eq!(read_pid_file(&path), Some(4321));
    assert_eq!(agent_pid(&path, Some(7)), Some(7));
    assert_eq!(agent_pid(&path, None), Some(4321));
}

#[test]
fn cli_sessions_skips_agent_pid_and_bad_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let grok = dir.path();
    assert!(cli_sessions(grok, None).is_empty());

    fs::write(grok.join("active_sessions.json"), "not-json").expect("write");
    assert!(cli_sessions(grok, None).is_empty());

    fs::write(
        grok.join("active_sessions.json"),
        r#"[{"session_id":"abc","pid":1}]"#,
    )
    .expect("write");
    let skip = cli_sessions(grok, Some(1));
    assert!(skip.is_empty());
}
