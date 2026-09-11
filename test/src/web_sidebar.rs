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
fn collapsed_sidebar_is_icon_rail_on_every_width() {
    let css = web_file("src/styles/sidebar.css");
    assert!(
        css.contains("html[data-sidebar=\"collapsed\"] #sidebar") && css.contains("width: 56px"),
        "collapsed sidebar must stay a 56px rail:\n{css}"
    );
    assert!(
        css.contains("html[data-sidebar=\"collapsed\"] .side-head")
            && css.contains("display: contents;"),
        "collapsed rail must flatten wrappers at every width:\n{css}"
    );
    assert!(
        !css.contains("@media (min-width: 901px)"),
        "icon-rail flattening must not be desktop-only:\n{css}"
    );
}

#[test]
fn no_hamburger_or_offcanvas_sidebar() {
    let theme = web_file("src/styles/theme-base.css");
    let layout = web_file("src/styles/layout.css");
    let app = web_file("src/App.jsx");
    let js = web_file("src/features/sidebar.js");
    let html = web_file("index.html");
    for (name, src) in [
        ("theme-base", &theme),
        ("layout", &layout),
        ("app", &app),
        ("sidebar.js", &js),
        ("index", &html),
    ] {
        assert!(
            !src.contains("open-side")
                && !src.contains("mobile-open")
                && !src.contains("i-menu")
                && !src.contains("translateX(-105%)"),
            "{name} must not keep the hamburger drawer:\n{src}"
        );
    }
    assert!(
        !js.contains("closeMobile")
            && !js.contains("openMobile")
            && !js.contains("openOverlay")
            && !js.contains("getElementById(\"scrim\")"),
        "sidebar must not drive a mobile overlay:\n{js}"
    );
    assert!(
        !app.contains("id=\"scrim\"") && !layout.contains("#scrim"),
        "sidebar scrim must be gone"
    );
}

fn fn_body(src: &str, name: &str) -> String {
    let needle = format!("function {name}(");
    let rest = src
        .split(&needle)
        .nth(1)
        .unwrap_or_else(|| panic!("missing {name}"));
    let start = rest.find('{').expect("function block");
    let mut depth = 0i32;
    for (i, c) in rest[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return rest[..=start + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unclosed {name}");
}

fn brace_after(src: &str, needle: &str) -> String {
    let rest = src
        .split(needle)
        .nth(1)
        .unwrap_or_else(|| panic!("missing {needle}"));
    let start = rest.find('{').expect("block");
    let mut depth = 0i32;
    for (i, c) in rest[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return rest[..=start + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unclosed after {needle}");
}

#[test]
fn collapse_click_only_toggles_rail() {
    let js = web_file("src/features/sidebar.js");
    let click = brace_after(&js, "collapseSideBtn.addEventListener(\"click\"");
    assert!(
        click.contains(
            "setSidebarCollapsed(document.documentElement.dataset.sidebar !== \"collapsed\")"
        ) && click.contains("syncCollapseTip()"),
        "collapse click must toggle the icon rail:\n{click}"
    );
    assert!(
        !click.contains("matchMedia"),
        "collapse click must not branch on viewport:\n{click}"
    );
}

#[test]
fn narrow_viewport_auto_collapses_icon_rail() {
    let js = web_file("src/features/sidebar.js");
    let apply = fn_body(&js, "applySidebarForViewport");
    assert!(
        apply.contains("matchMedia(\"(max-width: 900px)\")")
            && apply.contains("dataset.sidebar = \"collapsed\""),
        "crossing below 900px must collapse to the icon rail:\n{apply}"
    );
    assert!(
        apply.contains("localStorage.getItem(SIDE_KEY) === \"open\""),
        "leaving the narrow breakpoint must restore the stored preference:\n{apply}"
    );
    assert!(
        js.contains("addEventListener(\"change\", applySidebarForViewport)")
            && !js.contains("openMobile")
            && !js.contains("i-menu"),
        "viewport collapse must listen for width changes without a hamburger:\n{js}"
    );
}

#[test]
fn boot_uses_icon_rail_unless_user_opened_on_desktop() {
    let html = web_file("index.html");
    assert!(
        html.contains("side !== \"open\" || matchMedia(\"(max-width: 900px)\").matches")
            && html.contains("dataset.sidebar = \"collapsed\""),
        "narrow and first visit must boot into the icon rail:\n{html}"
    );
}

#[test]
fn collapsed_rail_hides_brand_and_matches_expanded_chrome() {
    let css = web_file("src/styles/sidebar.css");
    assert!(
        css.contains("html[data-sidebar=\"collapsed\"] .brand,")
            && css.contains("display: none !important;"),
        "collapsed rail must hide the brand mark until expanded:\n{css}"
    );
    assert!(
        !css.contains("html[data-sidebar=\"collapsed\"] .brand { order:"),
        "collapsed rail must not keep the brand in the icon stack:\n{css}"
    );
    assert!(
        css.contains("html[data-sidebar=\"collapsed\"] #collapse-side { order: 1; }")
            && css.contains("html[data-sidebar=\"collapsed\"] #new-session { order: 2; }")
            && css.contains("html[data-sidebar=\"collapsed\"] #search-btn { order: 3; }"),
        "collapse control must sit at the top of the collapsed rail:\n{css}"
    );
    assert!(
        css.contains("html[data-sidebar=\"collapsed\"] #ext-btn {")
            && css.contains("order: 4;")
            && css.contains("margin-top: auto;"),
        "ext nav must sit second-from-bottom like the expanded footer:\n{css}"
    );
    let tokens = web_file("src/tokens.css");
    assert!(
        tokens.contains("html[data-sidebar=\"collapsed\"] #ext-btn.side-nav {")
            && tokens.contains("margin-top: auto;"),
        "token overrides must not flatten ext to the top of the rail:\n{tokens}"
    );
    assert!(
        !tokens.contains("html[data-sidebar=\"collapsed\"] #ext-btn.side-nav")
            || !tokens
                .split("html[data-sidebar=\"collapsed\"] #new-session.side-nav")
                .nth(1)
                .unwrap_or("")
                .contains("margin: 0 auto"),
        "collapsed ext must not reset margin-top via shorthand:\n{tokens}"
    );
    assert!(
        css.contains("html[data-sidebar=\"collapsed\"] .foot-quota-row { order: 5; }"),
        "quota must sit at the bottom of the collapsed rail:\n{css}"
    );
}
