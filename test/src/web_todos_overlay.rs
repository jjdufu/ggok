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

#[test]
fn jump_bottom_has_no_tooltip_and_hover_border_only() {
    let app = web_file("src/App.jsx");
    let btn = app
        .split("id=\"jump-bottom\"")
        .nth(1)
        .and_then(|rest| rest.split('>').next())
        .expect("jump-bottom tag");
    assert!(
        !btn.contains("data-i18n-title"),
        "jump-bottom must not install a hover tip: {btn}"
    );
    assert!(
        !btn.contains("data-tip"),
        "jump-bottom must not ship data-tip: {btn}"
    );

    let js = web_file("src/features/timeline.js");
    assert!(
        !js.contains("setTip(jumpBottomBtn"),
        "jump-bottom must not call setTip"
    );

    let css = web_file("src/styles/composer.css");
    let base = css_block(&css, ".jump-bottom {");
    assert_decl(&base, "border:", "transparent");
    assert!(
        !base.contains("box-shadow: var(--shadow)"),
        "default jump-bottom should not use the raised shadow"
    );

    let hover = css_block(&css, ".jump-bottom:hover,");
    assert_decl(&hover, "border-color:", "var(--composer-border)");
    assert!(
        !hover.contains("var(--hover)"),
        "jump-bottom hover must not use transparent --hover fill:\n{hover}"
    );
}

#[test]
fn ctx_chip_shows_percent_by_default_and_detail_on_hover() {
    let css = web_file("src/styles/composer.css");
    let label = css_block(&css, ".composer-ctx-bar .ctx-label {");
    assert!(
        !label.contains("opacity: 0"),
        "percent label must stay visible when collapsed:\n{label}"
    );
    assert!(
        label.contains("var(--muted)"),
        "default ctx text must match todos muted:\n{label}"
    );

    let detail = css_block(&css, ".composer-ctx-bar .ctx-detail {");
    assert_decl(&detail, "max-width:", "0");
    assert_decl(&detail, "opacity:", "0");

    let hover = css_block(&css, ".composer-ctx-bar:hover .ctx-detail,");
    assert_decl(&hover, "max-width:", "160px");
    assert_decl(&hover, "opacity:", "1");

    let track = css_block(&css, ".composer-ctx-bar .ctx-track {");
    assert!(
        !track.contains("height: 4px"),
        "collapsed ctx chip must not be a 4px unlabeled track:\n{track}"
    );
    let todos_toggle = css_block(&css, ".todos-toggle {");
    assert!(
        track.contains("color-mix(in oklab, var(--fg) 10%, var(--composer))"),
        "ctx chip chrome must match todos:\n{track}"
    );
    assert!(
        todos_toggle.contains("color-mix(in oklab, var(--fg) 10%, var(--composer))"),
        "todos toggle chrome missing:\n{todos_toggle}"
    );

    let fill = css_block(&css, ".composer-ctx-bar .ctx-fill {");
    assert_decl(&fill, "opacity:", "0");
    let fill_hover = css_block(&css, ".composer-ctx-bar:hover .ctx-fill,");
    assert_decl(&fill_hover, "opacity:", "0.82");
    let label_hover = css_block(&css, ".composer-ctx-bar:hover .ctx-label,");
    assert!(
        label_hover.contains("#fff"),
        "hover ctx text must stay white on the fill:\n{label_hover}"
    );

    let js = web_file("src/features/sidebar.js");
    assert!(js.contains("querySelector(\".ctx-pct\")"));
    assert!(js.contains("querySelector(\".ctx-detail\")"));
    assert!(js.contains("pct + \"%\""));

    let app = web_file("src/App.jsx");
    assert!(app.contains("className=\"ctx-pct\""));
    assert!(app.contains("className=\"ctx-detail\""));
}

#[test]
fn ctx_chip_click_opens_session_usage_popover() {
    let app = web_file("src/App.jsx");
    let bar = app
        .split("id=\"ctx-bar\"")
        .nth(1)
        .and_then(|rest| rest.split("id=\"chips\"").next())
        .expect("ctx-bar block");
    assert!(bar.contains("id=\"ctx-usage\""), "usage popover must live on the ctx chip");
    assert!(bar.contains("id=\"usage-body\""), "usage-body must move onto the ctx chip");
    assert!(bar.contains("sessionUsage"), "{bar}");

    let drawer = app
        .split("id=\"drawer-status\"")
        .nth(1)
        .and_then(|rest| rest.split("id=\"drawer-files\"").next())
        .expect("drawer-status");
    assert!(
        !drawer.contains("id=\"usage-body\""),
        "session usage must leave the status drawer"
    );

    let css = web_file("src/styles/composer.css");
    let pop = css_block(&css, ".ctx-usage {");
    assert_decl(&pop, "position:", "absolute");
    assert_decl(&pop, "display:", "none");
    assert_decl(&pop, "bottom:", "calc(100% + 8px)");
    assert_decl(&pop, "min-width:", "320px");
    let open = css_block(&css, ".composer-ctx-bar.open .ctx-usage {");
    assert_decl(&open, "display:", "flex");

    let js = web_file("src/features/sidebar.js");
    assert!(js.contains("function setCtxUsageOpen"));
    assert!(js.contains("setCtxUsageOpen(!ctx.ctxUsageOpen)"));
    assert!(
        !js.contains("setTip(ctxBar") && !js.contains("setTip(ctxTrack"),
        "ctx usage popover must not use the button tooltip layer"
    );
}

#[test]
fn ctx_usage_models_use_picker_names_and_own_rows() {
    let js = web_file("src/features/drawer.js");
    assert!(
        js.contains("modelNameById"),
        "usage rows must resolve picker display names, not raw ids"
    );
    assert!(
        !js.contains("usageCells(box, m.model"),
        "model rows must not dump the raw model id into usageCells:\n{js}"
    );
    assert!(
        js.contains("box.className = \"usage-models\""),
        "each recorded model still gets its own row in usage-models"
    );

    let helpers = web_file("src/lib/helpers.js");
    assert!(helpers.contains("export function modelDisplayName"));
    assert!(helpers.contains("export function modelNameById"));
    assert!(
        helpers.contains("m.name || m.id"),
        "picker and usage must share the short name"
    );

    let menu = web_file("src/features/model-menu.js");
    assert!(
        menu.contains("modelDisplayName") && !menu.contains("function modelDisplayName"),
        "model menu must reuse helpers.modelDisplayName"
    );

    let css = web_file("src/styles/drawer.css");
    let models = css_block(&css, ".usage-models {");
    assert!(
        models.contains("max-content minmax(0, 1fr)"),
        "model rows need two columns so the short name is not ellipsized:\n{models}"
    );
    assert!(
        !models.contains("max-content max-content"),
        "model rows must not keep the three-column usage grid:\n{models}"
    );
}
