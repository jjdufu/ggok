use std::fs;
use std::path::PathBuf;

fn web_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
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

#[test]
fn latest_version_is_plain_text_not_a_link() {
    let app = web_file("src/App.jsx");
    assert!(
        app.contains(
            "<span id=\"quota-ver-latest\" className=\"quota-row-v quota-ver-latest\"></span>"
        ),
        "latest version must be a non-interactive span:\n{app}"
    );
    assert!(
        !app.contains("id=\"quota-ver-latest\" className=\"quota-ver-latest\"></button>"),
        "latest version must not stay a button:\n{app}"
    );

    let js = web_file("src/features/version.js");
    assert!(
        !js.contains("window.open")
            && !js.contains("LATEST_RELEASE")
            && !js.contains("addEventListener(\"click\""),
        "latest version must not navigate to GitHub:\n{js}"
    );
}

#[test]
fn latest_version_falls_back_to_current_when_unknown() {
    let js = web_file("src/features/version.js");
    let paint = fn_body(&js, "paint");
    assert!(
        paint.contains("latestEl.textContent = latest || cur")
            && paint.contains("const cur = ver ? fmtVer(ver) : lastCur")
            && paint.contains("if (curEl) curEl.textContent = cur"),
        "unknown latest must show the current version, not a blank:\n{paint}"
    );

    let refresh = fn_body(&js, "refreshVersion");
    assert!(
        refresh.contains("if (lastCur) paint({ version: lastCur })"),
        "a failed look must keep the current version on screen:\n{refresh}"
    );
}

#[test]
fn latest_version_turns_red_when_update_available() {
    let js = web_file("src/features/version.js");
    assert!(
        js.contains("st.update_available")
            && js.contains("classList.toggle(\"new\", !!(st && st.update_available))"),
        "paint must mark a newer latest in red:\n{js}"
    );

    let css = web_file("src/styles/sidebar.css");
    assert!(
        css.contains(".quota-ver-latest.new {") && css.contains("color: var(--danger)"),
        "a newer latest must use danger red:\n{css}"
    );
    assert!(
        !css.contains(".quota-ver-latest.has"),
        "latest must not keep the clickable .has treatment:\n{css}"
    );
}

#[test]
fn version_check_reuses_quota_look() {
    let quota = web_file("src/features/quota.js");
    let look = fn_body(&quota, "refreshAccountLook");
    assert!(
        look.contains("refreshAccount()") && look.contains("ctx.refreshVersion()"),
        "hover look must also query version:\n{look}"
    );

    let js = web_file("src/features/version.js");
    assert!(
        js.contains("ctx.refreshVersion = refreshVersion") && js.contains("refreshVersion();"),
        "version bind must expose refresh for the quota look:\n{js}"
    );
    assert!(
        !js.contains("setTimeout(refreshVersion") && !js.contains("quotaBtn"),
        "version must not keep a private click/timer query:\n{js}"
    );
}
