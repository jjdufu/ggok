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
fn start_dir_does_not_auto_select_first_root() {
    let js = web_file("src/features/dir-modal.js");
    assert!(
        !js.contains("dirSel = paths[0]"),
        "start list must not pre-select the first root:\n{js}"
    );
    assert!(
        js.contains("if (!paths.includes(dirSel)) dirSel = \"\""),
        "stale start selection must clear instead of falling back to paths[0]:\n{js}"
    );
}

#[test]
fn start_dir_rows_do_not_use_path_tooltips() {
    let js = web_file("src/features/dir-modal.js");
    assert!(
        !js.contains("setTip(b, up ? t(\"wsUp\") : path || name)"),
        "dir rows must not hover-tip the same path already shown as subtitle:\n{js}"
    );
    assert!(
        js.contains("setTip(b, t(\"wsUp\"))"),
        "parent row may keep a short up tip:\n{js}"
    );
    assert!(
        !js.contains("setTip(dirModalPath"),
        "the path label must not install a hover tip:\n{js}"
    );
    assert!(
        js.contains("aria-pressed"),
        "selected dir rows must expose pressed state:\n{js}"
    );
}

#[test]
fn dir_selected_style_is_not_hover() {
    let css = web_file("src/styles/dir-modal.css");
    assert!(
        !css.contains(".dir-item:hover, .dir-item.on"),
        "selected dir rows must not share the hover-only fill:\n{css}"
    );
    assert!(
        css.contains(".dir-item.on")
            && css.contains("var(--active)")
            && css.contains("var(--border-strong)"),
        "selected dir rows need a stronger fill and inset ring:\n{css}"
    );
}
