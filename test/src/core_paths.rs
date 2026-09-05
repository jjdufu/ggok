use ggok_core::paths::{
    compress_upload, cwd_allowed, expand_roots, is_under, resolve_existing_dir, under_any_root,
};
use std::fs;
use std::path::{Path, PathBuf};

fn canon_temp() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = fs::canonicalize(tmp.path()).expect("canonicalize tempdir");
    (tmp, path)
}

#[test]
fn expand_roots_skips_empty_relative_and_missing() {
    let (_tmp, path) = canon_temp();
    let abs = path.to_string_lossy().into_owned();
    let roots = expand_roots(Some(&[
        String::new(),
        "   ".into(),
        "relative/path".into(),
        "/no/such/ggok-root-dir".into(),
        abs.clone(),
        abs,
    ]));
    assert_eq!(roots, vec![path]);
}

#[test]
fn expand_roots_none_is_empty() {
    assert!(expand_roots(None).is_empty());
}

#[test]
fn is_under_is_component_wise() {
    let root = Path::new("/workspace");
    assert!(is_under(Path::new("/workspace"), root));
    assert!(is_under(Path::new("/workspace/src"), root));
    assert!(!is_under(Path::new("/workspace-other"), root));
    assert!(!is_under(Path::new("/tmp"), root));
}

#[test]
fn under_any_root_empty_means_any_absolute() {
    assert!(under_any_root(Path::new("/abs"), &[]));
    assert!(!under_any_root(Path::new("rel"), &[]));
    let root = PathBuf::from("/only");
    assert!(under_any_root(
        Path::new("/only/a"),
        std::slice::from_ref(&root)
    ));
    assert!(!under_any_root(Path::new("/other"), &[root]));
}

#[test]
fn resolve_and_allow_cwd() {
    let (_tmp, path) = canon_temp();
    let raw = path.to_str().expect("utf8 path");
    assert_eq!(resolve_existing_dir(raw).expect("dir"), path);
    assert!(resolve_existing_dir("").is_err());
    assert!(resolve_existing_dir("not-absolute").is_err());

    let roots = vec![path.clone()];
    assert_eq!(cwd_allowed(raw, &roots).expect("allowed"), path);

    let outside = tempfile::tempdir().expect("outside");
    let outside = fs::canonicalize(outside.path()).expect("canon outside");
    assert!(cwd_allowed(outside.to_str().expect("utf8"), &roots).is_err());
}

#[test]
fn compress_upload_leaves_non_png_alone() {
    let bytes = b"not-an-image".to_vec();
    assert_eq!(compress_upload("note.txt", bytes.clone()), bytes);
}
