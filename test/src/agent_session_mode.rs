use std::fs;
use std::path::PathBuf;

fn crate_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn fn_body(src: &str, sig: &str) -> String {
    let start = src
        .find(sig)
        .unwrap_or_else(|| panic!("missing {sig}"));
    let rest = &src[start..];
    let open = rest
        .find('{')
        .unwrap_or_else(|| panic!("missing block for {sig}"));
    let mut depth = 0i32;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return rest[..=open + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unclosed block for {sig}");
}

#[test]
fn set_session_mode_does_not_inject_slash_prompts() {
    let src = crate_file("crates/agent/src/session_control.rs");
    let body = fn_body(&src, "pub async fn set_session_mode");
    assert!(
        !body.contains("session/prompt"),
        "mode changes must not write /auto into the transcript:\n{body}"
    );
    assert!(
        !body.contains("\"/auto\"") && !body.contains("\"/always-approve\"") && !body.contains("\"/plan\""),
        "mode changes must not fall back to slash text:\n{body}"
    );
    assert!(
        body.contains("set_plan_mode") && body.contains("set_permission_mode"),
        "mode must go through ACP helpers:\n{body}"
    );
}

#[test]
fn next_prompt_carries_session_permission_meta() {
    let src = crate_file("crates/agent/src/session.rs");
    let body = fn_body(&src, "pub(crate) async fn start_prompt");
    assert!(
        body.contains("acp_session_meta"),
        "the following user turn should carry yolo/auto meta:\n{body}"
    );
}

#[test]
fn session_new_applies_requested_mode() {
    let src = crate_file("crates/agent/src/session.rs");
    let body = fn_body(&src, "pub async fn session_new");
    assert!(
        body.contains("resolve_create_mode"),
        "session/new must honor the UI mode, not only process config:\n{body}"
    );
    assert!(
        body.contains("acp_session_meta(meta_mode)"),
        "session/new meta must use the requested mode:\n{body}"
    );
    assert!(
        !body.contains("acp_session_meta(&self.permission_mode)"),
        "session/new must not ignore a caller-supplied mode:\n{body}"
    );
    assert!(
        body.contains("set_plan_mode") && body.contains("requested == \"plan\""),
        "plan on create must toggle plan mode after session/new:\n{body}"
    );

    let routes = crate_file("crates/server/src/routes/session.rs");
    assert!(
        routes.contains("pub mode: Option<String>"),
        "POST /api/sessions must accept mode:\n{routes}"
    );
    assert!(
        routes.contains("body.mode.as_deref()"),
        "create session must pass mode into session_new:\n{routes}"
    );
}
