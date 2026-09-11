use std::fs;
use std::path::PathBuf;

fn web_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn fn_body(src: &str, sig: &str) -> String {
    let start = src.find(sig).unwrap_or_else(|| panic!("missing {sig}"));
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
fn option_chords_are_not_eaten_by_the_prompt() {
    let src = web_file("src/PromptEditor.jsx");
    let detect = fn_body(&src, "function isOptionShortcut");
    assert!(
        detect.contains("event.altKey")
            && detect.contains("!event.metaKey")
            && detect.contains("!event.ctrlKey")
            && detect.contains("event.key !== \"Enter\""),
        "only bare Option/Alt chords (not Alt+Enter) should bypass the editor:\n{detect}"
    );

    let capture = fn_body(&src, "const onOptionKeyDownCapture");
    assert!(
        capture.contains("stopImmediatePropagation()"),
        "capture keydown must hide Option chords from ProseMirror:\n{capture}"
    );
    assert!(
        !capture.contains("preventDefault()"),
        "Option keydown must not preventDefault or OS shortcuts stay dead:\n{capture}"
    );

    let before = fn_body(&src, "const onOptionBeforeInput");
    assert!(
        before.contains("preventDefault()"),
        "beforeinput should block the Option character insert:\n{before}"
    );

    assert!(
        src.contains("addEventListener(\"keydown\", onOptionKeyDownCapture, true)"),
        "Option passthrough must bind in capture on the editor DOM"
    );
    assert!(
        src.contains("addEventListener(\"beforeinput\", onOptionBeforeInput)"),
        "Option passthrough must cancel the following beforeinput"
    );
}
