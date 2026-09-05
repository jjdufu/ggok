use ggok_core::workspace::{
    delete_workspace, list_workspace, resolve_workspace_dir, resolve_workspace_entry,
};
use std::fs;
use std::path::PathBuf;

struct Tree {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

fn tree() -> Tree {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(tmp.path()).expect("canon");
    fs::create_dir(root.join("src")).expect("mkdir src");
    fs::write(root.join("src/main.rs"), b"fn main() {}").expect("write");
    fs::write(root.join("README.md"), b"hi").expect("write readme");
    Tree { _tmp: tmp, root }
}

#[test]
fn resolve_inside_and_reject_escape() {
    let t = tree();
    let roots = [t.root.clone()];
    let cwd = t.root.to_str().expect("utf8");

    let dir = resolve_workspace_dir(cwd, "src", &roots).expect("src dir");
    assert_eq!(dir, t.root.join("src"));
    let file = resolve_workspace_entry(cwd, "src/main.rs", &roots).expect("file");
    assert_eq!(file, t.root.join("src/main.rs"));

    assert!(resolve_workspace_entry(cwd, "/etc/passwd", &roots).is_err());
    assert!(resolve_workspace_dir(cwd, "src/main.rs", &roots).is_err());
}

#[test]
fn list_and_delete_file_but_not_cwd() {
    let t = tree();
    let roots = [t.root.clone()];
    let cwd = t.root.to_str().expect("utf8");

    let listing = list_workspace(cwd, "", &roots).expect("list");
    assert!(listing.entries.iter().any(|e| e.name == "src" && e.dir));
    assert!(
        listing
            .entries
            .iter()
            .any(|e| e.name == "README.md" && !e.dir)
    );

    delete_workspace(cwd, "README.md", &roots).expect("delete file");
    assert!(!t.root.join("README.md").exists());
    assert!(delete_workspace(cwd, "", &roots).is_err());
    assert!(t.root.exists());
}
