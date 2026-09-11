use std::fs;
use std::path::PathBuf;

fn crate_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn update_prints_quiet_steps_without_bar() {
    let src = crate_file("crates/cli/src/update.rs");
    assert!(
        src.contains("println!(\"Updating {current} → {latest}\")")
            && src.contains("println!(\"Downloading\")")
            && src.contains("println!(\"Verifying\")")
            && src.contains("println!(\"Installing\")")
            && src.contains("println!(\"Updated to {latest}\")")
            && src.contains("println!(\"Restarted\")"),
        "update must print short steps and Restarted:\n{src}"
    );
    assert!(
        !src.contains("Verifying.")
            && !src.contains("Installing.")
            && !src.contains("stopped pid=")
            && !src.contains("started pid=")
            && !src.contains("stop_web(true)"),
        "update must not dump start/stop chrome or old dotted steps:\n{src}"
    );
    assert!(
        src.contains("Release {latest} is not ready yet")
            && !src.contains("Already up to date ({current})."),
        "missing assets must not pretend to be up to date:\n{src}"
    );
}

#[test]
fn update_maps_network_errors_without_curl_noise() {
    let src = crate_file("crates/cli/src/update.rs");
    assert!(
        src.contains("Network timeout while downloading {latest}")
            && src.contains("Check the connection and run ggok update again")
            && src.contains("Download was corrupted")
            && src.contains("Could not replace the ggok binary")
            && src.contains("Updated to {latest} but could not restart"),
        "update must map failures to short user lines:\n{src}"
    );
}

#[test]
fn curl_download_captures_stderr_and_skips_progress_bar() {
    let src = crate_file("crates/core/src/release.rs");
    assert!(
        !src.contains("--progress-bar")
            && !src.contains("eprintln!()")
            && !src.contains("IsTerminal"),
        "curl must not inherit a TTY progress bar:\n{src}"
    );
    assert!(
        src.contains("Always captures curl stderr")
            && src.contains("\"--connect-timeout\"")
            && src.contains("\"15\"")
            && src.contains("Network timeout while downloading")
            && src.contains("Could not reach GitHub"),
        "curl failures must map timeout/reachability without leaking curl text:\n{src}"
    );
    assert!(
        !src.contains("bail!(\"download failed\")") && !src.contains("download failed"),
        "generic download failed must be gone:\n{src}"
    );
}

#[test]
fn cli_errors_print_display_not_chain() {
    let src = crate_file("crates/cli/src/main.rs");
    assert!(
        src.contains("eprintln!(\"{e}\")") && !src.contains("{e:#}"),
        "cli must not dump anyhow chains:\n{src}"
    );
}
