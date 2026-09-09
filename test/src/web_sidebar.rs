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
fn mobile_drawer_does_not_override_collapsed_rail() {
    let css = web_file("src/styles/theme-base.css");
    assert!(
        css.contains("html:not([data-sidebar=\"collapsed\"]) #sidebar")
            && css.contains("transform: translateX(-105%)"),
        "off-canvas drawer must apply only while expanded:\n{css}"
    );
    assert!(
        css.contains("html:not([data-sidebar=\"collapsed\"]) #open-side"),
        "hamburger must hide on the collapsed rail:\n{css}"
    );
    assert!(
        !css.contains("html[data-sidebar=\"collapsed\"] #tree { display: block !important; }")
            && !css.contains(
                "html[data-sidebar=\"collapsed\"] .brand-text { display: inline !important; }"
            ),
        "mobile CSS must not restore the full sidebar while collapsed:\n{css}"
    );
}

#[test]
fn collapse_click_toggles_rail_on_mobile() {
    let js = web_file("src/features/sidebar.js");
    assert!(
        js.contains("const collapse = document.documentElement.dataset.sidebar !== \"collapsed\"")
            && js.contains("setSidebarCollapsed(collapse)")
            && js.contains("if (collapse) closeMobile()")
            && js.contains("else openMobile()"),
        "narrow screens must collapse to the icon rail, not only close the drawer:\n{js}"
    );
    assert!(
        !js.contains("closeMobile();\n        return;"),
        "collapse must not return early on mobile without toggling state:\n{js}"
    );
}
