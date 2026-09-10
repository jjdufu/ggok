use std::fs;
use std::path::PathBuf;

fn crate_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn start_stop_status_use_short_sentences() {
    let src = crate_file("crates/cli/src/ctl.rs");
    assert!(
        src.contains("Already running (pid {pid})")
            && src.contains("Started (pid {pid})")
            && src.contains("println!(\"Stopped\")")
            && src.contains("println!(\"Not running\")")
            && src.contains("Running (pid {pid})"),
        "ctl chrome must use short sentences:\n{src}"
    );
    assert!(
        !src.contains("stopped pid=")
            && !src.contains("killed pid=")
            && !src.contains("running pid={pid}")
            && !src.contains("already running pid="),
        "ctl must not keep pid= log lines:\n{src}"
    );
    assert!(
        src.contains("Sessions still running; leader left up")
            && src.contains("Could not stop the leader")
            && src.contains("Could not start\\nSee {}"),
        "stop --all and start failures must be plain English:\n{src}"
    );
}

#[test]
fn uninstall_and_status_hide_noise_by_default() {
    let src = crate_file("crates/cli/src/ctl.rs");
    assert!(
        src.contains("println!(\"Uninstalled\")")
            && src.contains("Uninstalled with leftovers:")
            && src.contains("Could not stop ggok before uninstall")
            && src.contains("Leader still running (pid {})"),
        "uninstall must use the short leftover copy:\n{src}"
    );
    assert!(
        src.contains("if web_pid.is_some() || verbose")
            && src.contains("if verbose {")
            && src.contains("print_leader_line()"),
        "status must keep leader pid=- off the default view:\n{src}"
    );
}
