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

#[test]
fn turn_map_is_overlay_sibling_not_inside_timeline() {
    let app = web_file("src/App.jsx");
    let shell = app
        .split("className=\"timeline-shell\"")
        .nth(1)
        .expect("timeline-shell");
    let timeline_at = shell.find("id=\"timeline\"").expect("timeline");
    let map_at = shell.find("id=\"turn-map\"").expect("turn-map");
    assert!(
        timeline_at < map_at,
        "turn-map must sit beside #timeline so it does not scroll with turns"
    );
    assert!(
        !shell.contains("id=\"turn-map-preview\""),
        "previews belong on each tick, not a singleton overlay"
    );
    let peek = app
        .split("id=\"peek-timeline\"")
        .nth(1)
        .expect("peek-timeline");
    assert!(
        !peek.contains("turn-map"),
        "peek sessions must not get a turn map"
    );
}

#[test]
fn turn_map_ticks_are_quiet_and_preview_matches_say() {
    let css = web_file("src/styles/chat.css");
    let map = css_block(&css, ".turn-map {");
    assert!(map.contains("position: absolute"), "{map}");
    assert!(
        !map.contains("right: 6px"),
        "map must sit inboard of the window edge:\n{map}"
    );
    assert!(map.contains("50%"), "map should track the centered transcript:\n{map}");

    let tick = css_block(&css, ".turn-map-tick {");
    assert!(tick.contains("pointer-events: auto"), "{tick}");
    assert!(
        !tick.contains("var(--hover)"),
        "ticks must not use transparent hover fill:\n{tick}"
    );

    let preview = css_block(&css, ".turn-map-preview {");
    assert!(preview.contains("var(--bubble)"), "{preview}");
    assert!(preview.contains("border-radius: 16px"), "{preview}");
    assert!(preview.contains("text-overflow: ellipsis"), "{preview}");
    let hover_all = css_block(&css, ".turn-map:hover .turn-map-preview,");
    assert!(hover_all.contains("opacity: 1"), "hovering the map must reveal every preview:\n{hover_all}");
    assert!(
        !preview.contains("counter-reset") && !preview.contains("list-style"),
        "preview must not look like a numbered directory:\n{preview}"
    );

    assert!(
        css.contains("@media (max-width: 900px)"),
        "turn map must hide on the existing mobile breakpoint"
    );
}

#[test]
fn turn_map_js_jumps_without_tooltips() {
    let js = web_file("src/features/timeline.js");
    assert!(js.contains("function syncTurnMap"));
    assert!(js.contains("function jumpToTurn"));
    assert!(js.contains("followOutput = false"));
    assert!(js.contains("behavior: \"auto\""));
    assert!(!js.contains("behavior: \"smooth\""));
    assert!(js.contains("marks.length < 2"));
    assert!(
        js.contains("(mapH - used) / 2"),
        "ticks must pack with even spacing, not stretch to session length"
    );
    assert!(
        !js.contains("m.top / span"),
        "tick y must not be mapped from document offset"
    );
    assert!(
        !js.contains("setTip(tick") && !js.contains("setTip(turnMap"),
        "turn map must not use the button tooltip layer"
    );
    assert!(js.contains("t(\"turnMapImage\")"));
    assert!(js.contains("t(\"turnMapFile\")"));
}

#[test]
fn turn_map_i18n_keys_exist() {
    let i18n = web_file("public/i18n.js");
    for key in ["turnMapAria", "turnMapImage", "turnMapFile"] {
        let hits = i18n.matches(&format!("{key}:")).count();
        assert!(hits >= 2, "{key} must exist in zh and en, found {hits}");
    }
}
