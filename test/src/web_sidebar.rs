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

#[test]
fn collapse_click_only_toggles_rail() {
    let js = web_file("src/features/sidebar.js");
    assert!(
        js.contains(
            "setSidebarCollapsed(document.documentElement.dataset.sidebar !== \"collapsed\")"
        ) && js.contains("syncCollapseTip()"),
        "collapse must toggle the icon rail at every width:\n{js}"
    );
    assert!(
        !js.contains("matchMedia(\"(max-width: 900px)\")"),
        "collapse must not branch on viewport:\n{js}"
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
