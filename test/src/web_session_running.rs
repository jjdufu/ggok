use std::fs;
use std::path::PathBuf;

fn web_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn sidebar_running_dot_clears_on_cancel_and_live_stop() {
    let sidebar = web_file("src/features/sidebar.js");
    assert!(
        sidebar.contains("function markSessionRunning")
            && sidebar.contains("s.running = !!on")
            && sidebar.contains("ctx.markSessionRunning = markSessionRunning"),
        "sidebar must expose an immediate running-dot updater:\n{sidebar}"
    );

    let composer = web_file("src/features/composer.js");
    assert!(
        composer.contains("markSessionRunning(ctx.currentId, false)")
            && composer.contains("function cancelRunning"),
        "stop must clear the sidebar running dot without waiting for SSE:\n{composer}"
    );
    assert!(
        composer.contains("markSessionRunning(id, !!ctx.running)"),
        "session pull must sync the sidebar running dot:\n{composer}"
    );

    let sse = web_file("src/features/sse.js");
    assert!(
        sse.contains("markSessionRunning(id, !!ctx.running)"),
        "live events must drive the sidebar running dot:\n{sse}"
    );
    assert!(
        sse.contains("markSessionRunning(id, false)"),
        "done/error must clear the sidebar running dot:\n{sse}"
    );
}
