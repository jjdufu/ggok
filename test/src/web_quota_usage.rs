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
fn usage_color_is_shared_green_yellow_red_gradient() {
    let helpers = web_file("src/lib/helpers.js");
    assert!(
        helpers.contains("export function usageColor(pct)"),
        "usageColor must be a shared helper:\n{helpers}"
    );
    assert!(
        helpers.contains("hue = 142 - (p / 50) * (142 - 45)")
            && helpers.contains("hue = 45 - ((p - 50) / 50) * (45 - 10)"),
        "shared tone must keep the ctx green→yellow→red ramp:\n{helpers}"
    );
    assert!(
        helpers.contains("n <= 0") && helpers.contains("return \"\""),
        "0% must fall back to gray by returning no tone:\n{helpers}"
    );

    let quota = web_file("src/features/quota.js");
    assert!(
        quota.contains("usageColor") && quota.contains("from \"../lib/helpers.js\""),
        "quota chip must reuse usageColor:\n{quota}"
    );
    assert!(
        !quota.contains("function quotaRed") && !quota.contains("quotaRed("),
        "quota must not keep a private red-only tone:\n{quota}"
    );

    let side = web_file("src/features/sidebar.js");
    assert!(
        side.contains("usageColor") && side.contains("from \"../lib/helpers.js\""),
        "ctx chip must reuse usageColor:\n{side}"
    );
    assert!(
        !side.contains("hue = 142") && !side.contains("hsl(${Math.round(hue)}"),
        "ctx chip must not keep a private hue ramp:\n{side}"
    );
}

#[test]
fn quota_and_ctx_keep_separate_pulse_thresholds() {
    let quota = web_file("src/features/quota.js");
    assert!(
        quota.contains("toggle(\"hot\", has && pct >= 90)")
            && quota.contains("toggle(\"pulse\", has && pct >= 98)"),
        "quota breathing must stay at 98% with hot from 90%:\n{quota}"
    );

    let side = web_file("src/features/sidebar.js");
    assert!(
        side.contains("toggle(\"warn\", pct >= 60 && pct < 80)")
            && side.contains("toggle(\"hot\", pct >= 80)"),
        "ctx occupancy thresholds must stay independent:\n{side}"
    );
    assert!(
        !side.contains("toggle(\"pulse\""),
        "ctx chip must not inherit quota breathing:\n{side}"
    );
}

#[test]
fn quota_poll_pauses_in_background_and_refreshes_on_show() {
    let quota = web_file("src/features/quota.js");
    assert!(
        quota.contains("const QUOTA_OK_MS = 60 * 1000"),
        "foreground poll must stay at 60s:\n{quota}"
    );
    assert!(
        quota.contains("function pageVisible()")
            && quota.contains("document.visibilityState === \"visible\""),
        "quota fetch must key off page visibility:\n{quota}"
    );
    assert!(
        quota.contains("if (!pageVisible()) return")
            && quota.contains("stopQuotaTimers()"),
        "hidden pages must skip fetch and timers:\n{quota}"
    );
    assert!(
        quota.contains("addEventListener(\"visibilitychange\", onPageShow)")
            && quota.contains("addEventListener(\"pageshow\"")
            && quota.contains("e.persisted")
            && quota.contains("refreshAccount()"),
        "becoming visible must query usage immediately:\n{quota}"
    );
}

#[test]
fn quota_chip_hover_and_click_refresh_usage() {
    let quota = web_file("src/features/quota.js");
    assert!(
        quota.contains("addEventListener(\"mouseenter\"")
            && quota.contains("addEventListener(\"click\""),
        "quota chip must listen for hover and click:\n{quota}"
    );
    let click = quota
        .split("addEventListener(\"click\"")
        .nth(1)
        .expect("quota click listener");
    assert!(
        click.contains("refreshAccount()"),
        "click must trigger a usage query:\n{click}"
    );
    let hover = quota
        .split("addEventListener(\"mouseenter\"")
        .nth(1)
        .expect("quota mouseenter listener");
    assert!(
        hover.contains("refreshAccount()"),
        "hover must trigger a usage query:\n{hover}"
    );
}

#[test]
fn quota_hover_does_not_override_usage_color() {
    let css = web_file("src/styles/sidebar.css");
    let hover = css_block(&css, ".quota-btn:hover,");
    assert!(
        !hover.contains("--quota-color"),
        "hover/open must not flatten quota color to fg:\n{hover}"
    );
    let btn = css_block(&css, ".quota-btn {");
    assert!(
        btn.contains("--quota-color: var(--muted)"),
        "0% quota must stay muted gray:\n{btn}"
    );
}
