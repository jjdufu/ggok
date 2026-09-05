use ggok_core::occupy::{
    ClassifyInput, LeaderRecord, LiveView, SESSION_BUSY, SessionOp, Source, classify, cli_sessions,
    cmdline_matches_grok, conflict_busy, first_reachable_leader_pid, is_auto_spawned_leader_cmd,
    is_ggok_spawned_leader_cmd, is_leader_server_cmd, is_noleader_stdio, is_stdio_client_cmd,
    is_tui_cmd, jsonl_running, leader_is_independent, parse_leader_list, read_leader_record,
    read_pid_file, read_web_active, s3_is_hard_foreign, should_cancel_web_peer, stdio_holds_leader,
    tui_held, web_active_path, write_leader_record, write_web_active,
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
        can_attach: false,
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
fn tui_cmd_and_hard_foreign() {
    let tui = b"/home/grok/.grok/bin/grok\0--permission-mode\0bypassPermissions\0";
    let noleader = b"grok\0agent\0--no-leader\0stdio\0";
    let stdio = b"grok\0agent\0--leader\0stdio\0";
    let leader = b"grok\0agent\0leader\0--no-exit-on-disconnect\0";
    assert!(is_tui_cmd(tui));
    assert!(!is_tui_cmd(noleader));
    assert!(!is_tui_cmd(stdio));
    assert!(!is_tui_cmd(leader));
    assert!(s3_is_hard_foreign(9, Some(3), true, tui));
    assert!(s3_is_hard_foreign(9, Some(3), false, tui));
    assert!(!s3_is_hard_foreign(3, Some(3), false, tui));
    assert!(s3_is_hard_foreign(9, Some(3), true, noleader));
    assert!(!s3_is_hard_foreign(9, Some(3), true, stdio));
}

#[test]
fn should_cancel_web_peer_skips_tui() {
    assert!(should_cancel_web_peer("aaa", "bbb", false));
    assert!(!should_cancel_web_peer("aaa", "aaa", false));
    assert!(!should_cancel_web_peer("aaa", "bbb", true));
}

#[test]
fn classify_s3_non_tui_with_can_attach_is_writable() {
    let s3 = HashMap::from([("s1".into(), 9_u32)]);
    let got = classify(&ClassifyInput {
        id: "s1",
        live: None,
        our_runtime_pid: Some(3),
        s3: &s3,
        leftover_noleader_alive: false,
        jsonl_running: false,
        can_attach: true,
    });
    let cmd = ggok_core::sys::pid_cmdline(9);
    if is_tui_cmd(&cmd) || is_noleader_stdio(&cmd) {
        assert_eq!(got.source, Source::Foreign);
        assert!(!got.writable);
    } else {
        assert_eq!(got.source, Source::Disk);
        assert!(got.writable);
        assert!(!conflict_busy(got, SessionOp::Prompt));
    }
}

#[test]
fn classify_jsonl_running_with_can_attach_is_writable() {
    let got = classify(&ClassifyInput {
        id: "s1",
        live: None,
        our_runtime_pid: Some(3),
        s3: &HashMap::new(),
        leftover_noleader_alive: false,
        jsonl_running: true,
        can_attach: true,
    });
    assert_eq!(got.source, Source::Disk);
    assert!(got.writable);
    assert!(got.running);
    assert!(!conflict_busy(got, SessionOp::Prompt));
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
fn leader_and_stdio_cmd_kinds() {
    let stdio = b"grok\0agent\0--leader\0--leader-socket\0/home/grok/.grok/leader.sock\0stdio\0";
    let owned = b"/home/grok/.grok/bin/grok\0agent\0leader\0--no-exit-on-disconnect\0--no-auto-update\0--leader-socket\0/home/grok/.grok/leader.sock\0";
    let auto = b"/home/grok/.grok/bin/grok\0agent\0leader\0--no-exit-on-disconnect\0--relay-on-demand\0--grok-ws-url\0wss://code.grok.com/ws/code-agent\0";
    assert!(is_stdio_client_cmd(stdio));
    assert!(!is_leader_server_cmd(stdio));
    assert!(!is_auto_spawned_leader_cmd(stdio));
    assert!(!is_ggok_spawned_leader_cmd(stdio));

    assert!(is_leader_server_cmd(owned));
    assert!(is_ggok_spawned_leader_cmd(owned));
    assert!(!is_auto_spawned_leader_cmd(owned));
    assert!(!is_stdio_client_cmd(owned));

    assert!(is_leader_server_cmd(auto));
    assert!(is_auto_spawned_leader_cmd(auto));
    assert!(!is_ggok_spawned_leader_cmd(auto));
    assert!(!is_stdio_client_cmd(auto));
}

#[test]
fn first_reachable_leader_pid_skips_unreachable() {
    assert_eq!(first_reachable_leader_pid(""), None);
    assert_eq!(first_reachable_leader_pid("not-json"), None);
    assert_eq!(first_reachable_leader_pid("[]"), None);
    assert_eq!(
        first_reachable_leader_pid(
            r#"[{"pid":1,"pidLive":null,"classification":"Unreachable","socketPath":"/tmp/x"}]"#
        ),
        None
    );
    assert_eq!(
        first_reachable_leader_pid(
            r#"[{"pid":70175,"pidFromLock":70175,"pidLive":70175,"classification":"Reachable","socketPath":"/home/grok/.grok/leader.sock"}]"#
        ),
        Some(70175)
    );
    assert_eq!(
        first_reachable_leader_pid(
            r#"{"leaders":[{"classification":"unreachable","pidLive":9},{"classification":"Reachable","pid_live":42}]}"#
        ),
        Some(42)
    );
    assert_eq!(
        first_reachable_leader_pid(r#"{"classification":"Reachable","pidLive":7}"#),
        Some(7)
    );
    let rows = parse_leader_list(
        r#"[{"classification":"Unreachable","pidLive":1},{"classification":"Reachable","pidLive":2}]"#,
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].pid_live, Some(1));
    assert_eq!(first_reachable_leader_pid(
        r#"[{"classification":"Unreachable","pidLive":1},{"classification":"Reachable","pidLive":2}]"#
    ), Some(2));
}

#[test]
fn leader_record_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("grok-leader.json");
    let rec = LeaderRecord {
        pid: 99,
        owned: true,
        socket: "/tmp/leader.sock".into(),
    };
    write_leader_record(&path, &rec).expect("write");
    assert_eq!(read_leader_record(&path), Some(rec));
}

#[test]
fn web_active_roundtrip_and_reject_bad_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let leader = dir.path().join("grok-leader.json");
    let path = web_active_path(&leader);
    assert!(path.ends_with("web-active.json"));
    let id = "01234567-89ab-4cde-8f01-23456789abcd";
    write_web_active(&path, id).expect("write");
    assert_eq!(read_web_active(&path).as_deref(), Some(id));
    assert!(write_web_active(&path, "not-a-uuid").is_err());
    assert!(!tui_held(&HashMap::new(), id, None));
}

#[test]
fn current_process_does_not_hold_a_grok_leader() {
    let pid = std::process::id();
    assert!(!stdio_holds_leader(pid));
    assert!(
        leader_is_independent(pid),
        "test process parent is not a grok stdio client"
    );
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
