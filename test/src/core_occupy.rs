use ggok_core::occupy::{
    ClassifyInput, LiveView, SESSION_BUSY, SessionOp, Source, classify, cli_sessions,
    cmdline_matches_grok, conflict_busy, is_noleader_stdio, jsonl_running, read_pid_file,
};
use std::collections::HashMap;
use std::fs::{self, File};
use std::time::{Duration, SystemTime};

fn live(loaded: bool, running: bool) -> LiveView {
    LiveView {
        loaded,
        running,
        model: "grok".into(),
        effort: "high".into(),
    }
}

fn occ(
    id: &str,
    live_view: Option<&LiveView>,
    our: Option<u32>,
    s3: &HashMap<String, u32>,
    leftover: bool,
    jsonl: bool,
) -> ggok_core::occupy::Occupancy {
    classify(&ClassifyInput {
        id,
        live: live_view,
        our_runtime_pid: our,
        s3,
        leftover_noleader_alive: leftover,
        jsonl_running: jsonl,
    })
}

#[test]
fn classify_attached_when_loaded() {
    let s3 = HashMap::from([("s1".into(), 1_u32)]);
    let got = occ("s1", Some(&live(true, true)), None, &s3, false, true);
    assert_eq!(got.source, Source::Attached);
    assert!(got.writable);
    assert!(got.running);

    let idle = occ("s1", Some(&live(true, false)), None, &s3, false, false);
    assert_eq!(idle.source, Source::Attached);
    assert!(idle.writable);
    assert!(!idle.running);
}

#[test]
fn classify_foreign_not_writable() {
    let s3 = HashMap::from([("s1".into(), 9_u32)]);
    let got = occ("s1", Some(&live(false, false)), Some(3), &s3, false, false);
    assert_eq!(got.source, Source::Foreign);
    assert!(!got.writable);
    assert!(got.running);
    assert!(conflict_busy(got, SessionOp::Prompt));
    assert!(conflict_busy(got, SessionOp::Load));
    assert!(conflict_busy(got, SessionOp::Cancel));
    assert!(conflict_busy(got, SessionOp::Control));
    assert!(conflict_busy(got, SessionOp::Delete));
}

#[test]
fn classify_disk_writable() {
    let got = occ(
        "missing",
        None,
        None,
        &HashMap::<String, u32>::new(),
        false,
        false,
    );
    assert_eq!(got.source, Source::Disk);
    assert!(got.writable);
    assert!(!got.running);
    assert!(!conflict_busy(got, SessionOp::Prompt));
    assert!(conflict_busy(got, SessionOp::Load));
    assert!(conflict_busy(got, SessionOp::Cancel));
    assert!(!conflict_busy(got, SessionOp::Delete));
}

#[test]
fn classify_own_pid_in_s3_is_not_foreign() {
    let s3 = HashMap::from([("s1".into(), 42_u32)]);
    let got = occ("s1", None, Some(42), &s3, false, false);
    assert_eq!(got.source, Source::Disk);
    assert!(got.writable);
}

#[test]
fn classify_leftover_existing_id_is_observe() {
    let got = occ("s1", None, None, &HashMap::new(), true, true);
    assert_eq!(got.source, Source::Observe);
    assert!(!got.writable);
    assert!(got.running);
    assert!(conflict_busy(got, SessionOp::Prompt));
    assert!(conflict_busy(got, SessionOp::Delete));

    let idle = occ("s1", None, None, &HashMap::new(), true, false);
    assert_eq!(idle.source, Source::Observe);
    assert!(!idle.writable);
    assert!(!idle.running);
    assert!(!conflict_busy(idle, SessionOp::Delete));
}

#[test]
fn classify_jsonl_running_without_attach_is_observe() {
    let got = occ("s1", None, None, &HashMap::new(), false, true);
    assert_eq!(got.source, Source::Observe);
    assert!(!got.writable);
    assert!(got.running);
}

#[test]
fn leftover_idle_returns_to_disk() {
    let got = occ("s1", None, None, &HashMap::new(), false, false);
    assert_eq!(got.source, Source::Disk);
    assert!(got.writable);
}

#[test]
fn source_as_str() {
    assert_eq!(Source::Attached.as_str(), "attached");
    assert_eq!(Source::Observe.as_str(), "observe");
    assert_eq!(Source::Foreign.as_str(), "foreign");
    assert_eq!(Source::Disk.as_str(), "disk");
}

#[test]
fn session_busy_token() {
    assert_eq!(SESSION_BUSY, "session_busy");
}

#[test]
fn cmdline_match_and_stale() {
    assert!(cmdline_matches_grok(b"grok\0agent\0--no-leader\0stdio\0"));
    assert!(is_noleader_stdio(b"grok\0agent\0--no-leader\0stdio\0"));
    assert!(cmdline_matches_grok(b"/usr/bin/grok\0"));
    assert!(!is_noleader_stdio(b"/usr/bin/grok\0"));
    assert!(cmdline_matches_grok(
        b"grok\0agent\0--leader\0--leader-socket\0/tmp/x\0stdio\0"
    ));
    assert!(cmdline_matches_grok(b"grok\0agent\0leader\0"));
    assert!(!cmdline_matches_grok(b"/usr/lib/systemd/systemd\0"));
    assert!(!is_noleader_stdio(b"/sbin/init\0"));
}

#[test]
fn read_pid_file_parses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("agent.pid");
    assert!(read_pid_file(&path).is_none());
    fs::write(&path, "  4321 \n").expect("write pid");
    assert_eq!(read_pid_file(&path), Some(4321));
}

#[test]
fn cli_sessions_skips_stale_cmdline_and_bad_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let grok = dir.path();
    assert!(cli_sessions(grok).is_empty());

    fs::write(grok.join("active_sessions.json"), "not-json").expect("write");
    assert!(cli_sessions(grok).is_empty());

    fs::write(
        grok.join("active_sessions.json"),
        r#"[{"session_id":"abc","pid":1}]"#,
    )
    .expect("write");
    let skip = cli_sessions(grok);
    assert!(
        skip.is_empty(),
        "pid 1 is alive but cmdline is not grok, must be stale"
    );
}

fn write_jsonl(dir: &std::path::Path, kind: &str) {
    fs::create_dir_all(dir).expect("mkdir");
    let line = format!(
        r#"{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"{kind}"}}}}}}"#
    );
    fs::write(dir.join("updates.jsonl"), format!("{line}\n")).expect("write jsonl");
}

#[test]
fn jsonl_running_turn_completed_is_false() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sess = dir.path().join("s");
    write_jsonl(&sess, "turn_completed");
    assert!(!jsonl_running(&sess));
}

#[test]
fn jsonl_running_recent_other_is_true() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sess = dir.path().join("s");
    write_jsonl(&sess, "agent_message_chunk");
    let path = sess.join("updates.jsonl");
    let f = File::options().write(true).open(&path).expect("open jsonl");
    f.set_modified(SystemTime::now()).expect("mtime now");
    assert!(jsonl_running(&sess));
}

#[test]
fn jsonl_running_old_mtime_is_false() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sess = dir.path().join("s");
    write_jsonl(&sess, "agent_message_chunk");
    let path = sess.join("updates.jsonl");
    let f = File::options().write(true).open(&path).expect("open jsonl");
    f.set_modified(SystemTime::now() - Duration::from_secs(30))
        .expect("mtime old");
    assert!(!jsonl_running(&sess));
}

#[test]
fn jsonl_running_missing_file_is_false() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(!jsonl_running(&dir.path().join("nope")));
}
