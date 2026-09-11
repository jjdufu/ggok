use std::fs;
use std::path::PathBuf;

fn ext_js() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web/src/features/ext-modal.js");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn load_catch(src: &str, name: &str) -> String {
    let needle = format!("async function {name}");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {name}"));
    let rest = &src[start..];
    let catch_at = rest
        .find("} catch")
        .unwrap_or_else(|| panic!("{name} has no catch"));
    let after = &rest[catch_at..];
    let end = after
        .find("\n  async function")
        .or_else(|| after.find("\n  function "))
        .unwrap_or(after.len().min(360));
    after[..end].to_string()
}

#[test]
fn ext_list_loaders_do_not_toast_raw_cwd_errors() {
    let src = ext_js();
    for name in [
        "loadMcps",
        "loadPlugins",
        "loadSkills",
        "loadHooks",
        "loadWorkflows",
        "loadAgents",
    ] {
        let catch = load_catch(&src, name);
        assert!(
            !catch.contains("toast("),
            "{name} list load must fail quietly, not toast cwd errors:\n{catch}"
        );
    }
    assert!(
        !src.contains(".catch((e) => toast(e))"),
        "skill detail fetch must not toast raw errors"
    );
}

#[test]
fn ext_user_actions_still_toast() {
    let src = ext_js();
    assert!(src.contains("toast(t(\"pluginInstallOk\")"));
    assert!(src.contains("toast(t(\"mcpAddNeed\"))"));
    assert!(src.contains("toast(t(\"pluginAddNeed\"))"));
    assert!(src.contains("toast(t(\"skillCreateNeed\"))"));
}
