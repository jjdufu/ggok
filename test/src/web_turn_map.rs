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
        !shell.contains("turn-map-panel") && !shell.contains("turn-map-preview"),
        "the conversation list is created in JS, not baked into App.jsx"
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
#[allow(clippy::too_many_lines)]
fn turn_map_ticks_are_quiet_and_preview_matches_say() {
    let css = web_file("src/styles/chat.css");
    let map = css_block(&css, ".turn-map {");
    assert!(map.contains("position: absolute"), "{map}");
    assert!(
        !map.contains("right: 6px"),
        "map must sit inboard of the window edge:\n{map}"
    );
    assert!(map.contains("50%"), "map should track the centered transcript:\n{map}");
    assert!(
        map.contains("352px") && map.contains("clamp(24px, 3.2vw, 44px)"),
        "map gutter should track the 704px column with a flexible inset:\n{map}"
    );
    assert!(
        !map.contains("370px"),
        "old 370px offset glued the ticks to the transcript:\n{map}"
    );
    assert!(
        map.contains("352px") && map.contains("clamp(24px, 3.2vw, 44px)"),
        "map gutter must track the 704px column with a responsive inset:\n{map}"
    );
    assert!(
        !map.contains("370px"),
        "map must not sit 18px off the transcript:\n{map}"
    );

    assert!(
        map.contains("pointer-events: none"),
        "empty map column must not capture hover:\n{map}"
    );

    let tick = css_block(&css, ".turn-map-tick {");
    assert!(tick.contains("pointer-events: auto"), "{tick}");
    assert!(
        !tick.contains("var(--hover)"),
        "ticks must not use transparent hover fill:\n{tick}"
    );

    let mark = css_block(&css, ".turn-map-tick::after {");
    assert!(
        mark.contains("width: 14px"),
        "default tick width must stay 14px:\n{mark}"
    );
    let active = css_block(&css, ".turn-map-tick:hover::after,");
    assert!(
        active.contains("background: var(--fg)"),
        "current tick must invert color:\n{active}"
    );
    assert!(
        !active.contains("width:"),
        "current tick must not grow longer than the rest:\n{active}"
    );
    assert!(
        active.contains(".turn-map-tick.lit::after"),
        "hovered conversation must light the matching tick:\n{active}"
    );

    let tail = css_block(&css, ".turn-map-tick.tail::after {");
    assert!(
        tail.contains("opacity: 0.55"),
        "idle last tick must be dimmer than the selected tick:\n{tail}"
    );

    let panel = css_block(&css, ".turn-map-panel {");
    assert!(
        panel.contains("border: 0") && !panel.contains("border: 1px"),
        "panel must not draw a wrapping rectangle:\n{panel}"
    );
    assert!(
        panel.contains("background: transparent"),
        "panel chrome must be gone:\n{panel}"
    );
    let fade_hit = css_block(&css, ".turn-map-panel::before,");
    assert!(
        fade_hit.contains("pointer-events: none"),
        "fade plate must not steal clicks:\n{fade_hit}"
    );
    let fade = css_block(&css, ".turn-map-panel::before {");
    assert!(
        fade.contains("var(--bg)") && fade.contains("rgba(var(--bg-rgb), 0)"),
        "occluder must use page bg and fade like composer-fade:\n{fade}"
    );
    assert!(
        panel.contains("padding: 0 20px 0 0"),
        "ticks and list must have a hover-safe gap:\n{panel}"
    );
    let list = css_block(&css, ".turn-map-list {");
    assert!(list.contains("gap: 8px"), "{list}");
    assert!(
        panel.contains("max-height: 100%"),
        "panel height must follow the page/rail, not a fixed inward box:\n{panel}"
    );
    assert!(
        !panel.contains("56vh") && !panel.contains("420px"),
        "panel must not cap to a small inward max-height:\n{panel}"
    );
    let out = css_block(&css, ".turn-map[data-side=\"out\"] .turn-map-panel {");
    assert!(
        out.contains("left: 22px") && out.contains("right: auto") && out.contains("padding: 0 0 0 20px"),
        "wide pages must open the list outward with the same tick gap:\n{out}"
    );
    assert!(
        !css.contains(".turn-map:hover .turn-map-preview"),
        "hovering empty map space must not open the list"
    );
    let open = css_block(&css, ".turn-map:has(.turn-map-tick:hover) .turn-map-panel,");
    assert!(open.contains("display: flex"), "{open}");
    let row = css_block(&css, ".turn-map-row {");
    assert!(
        !row.contains("border: 1px solid"),
        "conversation rows must not draw a border:\n{row}"
    );
    assert!(row.contains("border: 0"), "{row}");
    assert!(
        row.contains("border-radius: 999px"),
        "each row must be a capsule, not a rounded rect:\n{row}"
    );
    assert!(row.contains("text-overflow: ellipsis"), "{row}");
    let row_on = css_block(&css, ".turn-map-row:hover,");
    assert!(
        !row_on.contains("border-color"),
        "selected/hover rows must not grow a bright outline:\n{row_on}"
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
    assert!(js.contains("function animateTimelineJump"));
    assert!(js.contains("JUMP_MS"));
    assert!(js.contains("prefers-reduced-motion"));
    assert!(!js.contains("behavior: \"smooth\""));
    assert!(
        !js.contains("behavior: \"auto\""),
        "turn-map jump must animate, not snap with behavior auto:\n{js}"
    );
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
    assert!(js.contains("turn-map-panel"));
    assert!(js.contains("turn-map-list"));
    assert!(js.contains("turn-map-row"));
    assert!(js.contains("function setTurnMapLit"));
    assert!(js.contains("function placeTurnMapPanel"));
    assert!(js.contains("turnMapPickedKey"));
    assert!(js.contains("dataset.side"));
    assert!(js.contains("spaceRight"));
    assert!(js.contains("\"tail\""));
    assert!(
        !js.contains("turn-map-preview"),
        "previews moved off the ticks into the shared panel"
    );
}

#[test]
fn turn_map_i18n_keys_exist() {
    let i18n = web_file("public/i18n.js");
    for key in ["turnMapAria", "turnMapImage", "turnMapFile"] {
        let hits = i18n.matches(&format!("{key}:")).count();
        assert!(hits >= 2, "{key} must exist in zh and en, found {hits}");
    }
}
