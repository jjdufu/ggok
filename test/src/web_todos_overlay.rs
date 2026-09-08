use std::fs;
use std::path::PathBuf;

fn web_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn css_block(src: &str, selector: &str) -> String {
    let start = src
        .find(selector)
        .unwrap_or_else(|| panic!("missing selector {selector}"));
    let rest = &src[start..];
    let open = rest
        .find('{')
        .unwrap_or_else(|| panic!("missing block for {selector}"));
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
    panic!("unclosed block for {selector}");
}

fn assert_decl(block: &str, property: &str, needle: &str) {
    assert!(
        block.contains(property) && block.contains(needle),
        "{property} should include {needle:?} in:\n{block}"
    );
}

#[test]
fn todos_chip_is_left_overlay_like_ctx_bar() {
    let css = web_file("src/styles/composer.css");
    let todos = css_block(&css, ".todos-bar {");
    assert_decl(&todos, "position:", "absolute");
    assert_decl(&todos, "left:", "14px");
    assert_decl(&todos, "bottom:", "calc(100% + 6px)");

    let ctx = css_block(&css, ".composer-ctx-bar {");
    assert_decl(&ctx, "position:", "absolute");
    assert_decl(&ctx, "right:", "14px");
    assert_decl(&ctx, "bottom:", "calc(100% + 6px)");
    assert!(
        !ctx.contains("left: 14px"),
        "context bar must stay on the right"
    );
}

#[test]
fn todos_preview_stays_collapsed_until_hover_or_open() {
    let css = web_file("src/styles/composer.css");
    let preview = css_block(&css, ".todos-preview {");
    assert_decl(&preview, "max-width:", "0");
    assert_decl(&preview, "opacity:", "0");

    let hover = css_block(&css, ".todos-bar:hover .todos-preview,");
    assert_decl(&hover, "max-width:", "180px");
    assert_decl(&hover, "opacity:", "1");
}

#[test]
fn todos_list_is_popover_not_in_flow() {
    let css = web_file("src/styles/composer.css");
    let list = css_block(&css, ".todos-list {");
    assert_decl(&list, "position:", "absolute");
    assert_decl(&list, "display:", "none");
    assert_decl(&list, "bottom:", "calc(100% + 8px)");

    let chat = web_file("src/styles/chat.css");
    assert!(
        !chat.contains(".todos-bar {"),
        "flow-layout todos-bar rules must not remain in chat.css"
    );
}

#[test]
fn todos_markup_stays_a_composer_sibling() {
    let app = web_file("src/App.jsx");
    let wrap = app
        .split("className=\"composer-wrap\"")
        .nth(1)
        .expect("composer-wrap");
    let inner_at = wrap
        .find("className=\"composer-inner\"")
        .expect("composer-inner");
    let todos_at = wrap.find("id=\"todos-bar\"").expect("todos-bar");
    let ctx_at = wrap.find("id=\"ctx-bar\"").expect("ctx-bar");
    assert!(
        todos_at < inner_at && ctx_at < inner_at,
        "overlay chips must be siblings of composer-inner, not inside it"
    );
}

#[test]
fn todos_preview_is_always_in_dom_for_css_hover() {
    let js = web_file("src/features/mode.js");
    assert!(
        !js.contains("if (!ctx.todosOpen && current && current.content)"),
        "preview must not be omitted while the chip is collapsed"
    );
    assert!(js.contains("preview.className = \"todos-preview\""));
    assert!(js.contains("e.stopPropagation()"));
}
