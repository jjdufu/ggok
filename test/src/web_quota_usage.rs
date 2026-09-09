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
        quota.contains("if (!pageVisible()) return") && quota.contains("stopQuotaTimers()"),
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

fn listener_body(src: &str, event: &str) -> String {
    let needle = format!("addEventListener(\"{event}\"");
    let rest = src
        .split(&needle)
        .nth(1)
        .unwrap_or_else(|| panic!("missing {event} listener"));
    let start = rest.find('{').expect("listener block");
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
    panic!("unclosed {event} listener");
}

#[test]
fn quota_chip_look_refreshes_usage_click_only_opens() {
    let quota = web_file("src/features/quota.js");
    assert!(
        quota.contains("const QUOTA_LOOK_MS = 2 * 1000")
            && quota.contains("function refreshAccountLook()")
            && quota.contains("now - lastLookAt < QUOTA_LOOK_MS"),
        "pointer/keyboard look must share a 2s cooldown:\n{quota}"
    );
    assert!(
        quota.contains("setInterval(refreshAccount, QUOTA_OK_MS)")
            && quota.contains("function onPageShow()")
            && quota.contains("refreshAccount();"),
        "60s poll and foreground resume must bypass look cooldown:\n{quota}"
    );

    let hover = listener_body(&quota, "mouseenter");
    assert!(
        hover.contains("refreshAccountLook()"),
        "hover must query through the look cooldown:\n{hover}"
    );

    let focus = listener_body(&quota, "focus");
    assert!(
        focus.contains(":focus-visible") && focus.contains("refreshAccountLook()"),
        "keyboard focus-visible must query through the look cooldown:\n{focus}"
    );
    assert!(
        !quota.contains("addEventListener(\"keydown\"") && !quota.contains("key === \"Tab\""),
        "must not bind the Tab key:\n{quota}"
    );

    let click = listener_body(&quota, "click");
    assert!(
        click.contains("setQuotaOpen") && !click.contains("refreshAccount"),
        "click must only toggle the popover:\n{click}"
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

#[test]
fn quota_fetch_paints_billing_window_even_without_used_percent() {
    let quota = web_file("src/features/quota.js");
    assert!(
        quota.contains("function hasBillingWindow(st)")
            && quota.contains("st.period_start || st.resets_at || st.period"),
        "reset payloads are identified by the billing window:\n{quota}"
    );
    assert!(
        quota.contains("accountReady(acc) || (acc && acc.ok !== false && hasBillingWindow(acc))"),
        "a new period without used_percent must still paint:\n{quota}"
    );
}

#[test]
fn quota_pop_anchors_to_rail_when_collapsed() {
    let quota = web_file("src/features/quota.js");
    let place = quota
        .split("function placeQuotaPop()")
        .nth(1)
        .expect("placeQuotaPop");
    assert!(
        place.contains("if (!collapsed)") && !place.contains("|| mobile"),
        "collapsed rail must keep the side popover on every width:\n{place}"
    );
}

#[test]
fn apply_account_drops_stale_percent_when_billing_window_changes() {
    let engine = web_file("src/engine.js");
    assert!(
        engine.contains("function sameBillingWindow(a, b)"),
        "account apply must compare billing windows:\n{engine}"
    );
    assert!(
        engine.contains("used_percent: 0") && engine.contains("remaining_percent: 100"),
        "a new period without usage must reset to 0%:\n{engine}"
    );
    assert!(
        engine.contains("if (sameBillingWindow(acc, prev))"),
        "same-period flakes may still reuse the last percent:\n{engine}"
    );
}
