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
fn mode_menu_lists_short_descriptions() {
    let js = web_file("src/features/mode.js");
    assert!(js.contains("modeAskDesc"));
    assert!(js.contains("modePlanDesc"));
    assert!(js.contains("modeAutoDesc"));
    assert!(js.contains("modeAlwaysDesc"));
    assert!(js.contains("mode-item-name"));
    assert!(js.contains("mode-item-desc"));
    assert!(
        !js.contains("b.textContent = t(key)"),
        "mode rows must not be a single unlabeled label:\n{js}"
    );

    let i18n = web_file("public/i18n.js");
    for key in [
        "modeAskDesc",
        "modePlanDesc",
        "modeAutoDesc",
        "modeAlwaysDesc",
    ] {
        let hits = i18n.matches(&format!("{key}:")).count();
        assert!(hits >= 2, "{key} must exist in zh and en, found {hits}");
    }
}

#[test]
fn occupy_toast_is_not_used_on_switch_or_send() {
    let composer = web_file("src/features/composer.js");
    assert!(composer.contains("discardInflight()"));
    assert!(
        !composer.contains("toast(t(occupyMessageKey"),
        "occupied sessions must use the banner, not a send toast:\n{composer}"
    );
    assert!(composer.contains("if (e && e.stale) return"));

    let api = web_file("src/lib/api.js");
    assert!(api.contains("export function discardInflight"));
    assert!(api.contains("err.stale = true"));
    assert!(api.contains("err.code = \"session_busy\""));

    let toast = web_file("src/lib/clipboard.js");
    assert!(
        toast.contains("sessionBusy"),
        "toast must drop sessionBusy so it does not overlay the composer:\n{toast}"
    );
    assert!(toast.contains("stale"));
}

#[test]
fn empty_session_can_pick_mode_before_id() {
    let js = web_file("src/features/mode.js");
    assert!(js.contains("function defaultMode"));
    assert!(js.contains("function applyLocalMode"));
    assert!(
        js.contains("if (!ctx.currentId)") && js.contains("applyLocalMode(mode)"),
        "empty composer must keep Ask/Plan/Auto/Always locally:\n{js}"
    );
    assert!(
        !js.contains("modeBtn.hidden = !ctx.currentId"),
        "mode button must stay visible on an empty session:\n{js}"
    );
    assert!(
        !js.contains("modeBtn.disabled = !ctx.currentId"),
        "mode button must not disable just because there is no session id:\n{js}"
    );
    let toggle_at = js.find("function toggleModeMenu").expect("toggleModeMenu");
    let toggle = js
        .get(toggle_at..toggle_at.saturating_add(500))
        .unwrap_or("");
    assert!(
        !toggle.contains("if (!ctx.currentId) return"),
        "empty session must open the mode menu:\n{toggle}"
    );

    let css = web_file("src/styles/composer.css");
    assert!(
        css.contains("#mode-btn[hidden]"),
        "hidden must actually hide #mode-btn under .composer-btn display"
    );

    let composer = web_file("src/features/composer.js");
    assert!(
        composer.contains("mode: ctx.mode || undefined"),
        "ensureSession must send the empty-session mode into POST /api/sessions:\n{composer}"
    );
    assert!(
        composer.contains("ctx.mode = ctx.defaultMode"),
        "new chat must reset mode to the runtime default, not the last session:\n{composer}"
    );

    let engine = web_file("src/engine.js");
    assert!(
        engine.contains("permission_mode") || engine.contains("ctx.defaultMode"),
        "boot must apply runtime permission_mode to an empty composer:\n{engine}"
    );
}
