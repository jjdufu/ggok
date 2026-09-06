use ggok_core::release::{
    asset_filename, asset_url, is_newer, os_arch, parse_latest_tag, parse_repo, parse_sha256sums,
    parse_version, replace_file_atomic, verify_file_sha256,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn parse_version_accepts_plain_v_and_pre() {
    let v = parse_version("0.1.3").expect("plain");
    assert_eq!(v.major, 0);
    assert_eq!(v.minor, 1);
    assert_eq!(v.patch, 3);
    assert_eq!(v.pre, None);

    let v = parse_version("v0.1.3").expect("v prefix");
    assert_eq!((v.major, v.minor, v.patch), (0, 1, 3));
    assert_eq!(v.pre, None);

    let v = parse_version("0.1.3-rc.1").expect("pre");
    assert_eq!(v.pre.as_deref(), Some("rc.1"));
}

#[test]
fn parse_version_rejects_short_latest_empty_and_build() {
    assert!(parse_version("1.2").is_err());
    assert!(parse_version("latest").is_err());
    assert!(parse_version("").is_err());
    assert!(parse_version("1.2.3+build").is_err());
}

#[test]
fn is_newer_table() {
    assert!(is_newer("0.1.4", "0.1.3"));
    assert!(!is_newer("0.1.3", "0.1.3"));
    assert!(is_newer("0.2.0", "0.1.9"));
    assert!(!is_newer("0.1.2", "0.1.3"));
    assert!(!is_newer("0.1.4-rc.1", "0.1.3"));
    assert!(is_newer("0.1.4-rc.1", "0.1.4-rc.0"));
    assert!(is_newer("0.1.3", "0.1.3-rc.1"));
    assert!(!is_newer("0.1.4-rc.10", "0.1.4-rc.9"));
}

#[test]
fn parse_latest_tag_from_github_url() {
    assert_eq!(
        parse_latest_tag("https://github.com/jjdufu/ggok/releases/tag/v0.1.3").expect("tag"),
        "0.1.3"
    );
    assert_eq!(
        parse_latest_tag("https://github.com/jjdufu/ggok/releases/tag/v0.1.3/").expect("slash"),
        "0.1.3"
    );
    assert_eq!(
        parse_latest_tag("https://github.com/jjdufu/ggok/releases/tag/0.1.3").expect("no v"),
        "0.1.3"
    );
    assert!(parse_latest_tag("https://github.com/jjdufu/ggok/releases/latest").is_err());
    assert!(parse_latest_tag("").is_err());
}

#[test]
fn asset_filename_is_concat() {
    assert_eq!(
        asset_filename("0.1.3", "linux", "amd64"),
        "ggok_0.1.3_linux_amd64.tar.gz"
    );
}

#[test]
fn parse_sha256sums_two_spaces_and_star() {
    let name = "ggok_0.1.3_linux_amd64.tar.gz";
    let two_space = format!("{EMPTY_SHA256}  {name}");
    let star = format!("{EMPTY_SHA256} *{name}");
    assert_eq!(
        parse_sha256sums(&two_space, name).expect("spaces"),
        EMPTY_SHA256
    );
    assert_eq!(parse_sha256sums(&star, name).expect("star"), EMPTY_SHA256);
    assert!(parse_sha256sums(&two_space, "other.tar.gz").is_err());
    assert!(parse_sha256sums("abcd  file.tar.gz", "file.tar.gz").is_err());
    assert!(parse_sha256sums(&format!("{EMPTY_SHA256}  {name}.bak"), name).is_err());
}

#[test]
fn os_arch_maps_this_host() {
    let (os, arch) = os_arch().expect("supported host");
    if cfg!(target_os = "linux") {
        assert_eq!(os, "linux");
    } else if cfg!(target_os = "macos") {
        assert_eq!(os, "darwin");
    } else {
        panic!("unexpected OS {os}");
    }
    if cfg!(target_arch = "x86_64") {
        assert_eq!(arch, "amd64");
    } else if cfg!(target_arch = "aarch64") {
        assert_eq!(arch, "aarch64");
    } else {
        panic!("unexpected arch {arch}");
    }
}

#[test]
fn parse_repo_accepts_default_rejects_traversal() {
    assert_eq!(parse_repo("jjdufu/ggok").expect("ok"), "jjdufu/ggok");
    assert!(parse_repo("jjdufu/..").is_err());
    assert!(parse_repo("../ggok").is_err());
    assert!(parse_repo("jjdufu/ggok.git").is_err());
    assert!(parse_repo("jjdufu/ggok/extra").is_err());
    assert!(parse_repo("jjdufu/").is_err());
    assert!(parse_repo("/ggok").is_err());
    assert!(parse_repo("").is_err());
}

#[test]
fn asset_url_requires_version_and_allow_list() {
    let url = asset_url("0.1.4", "linux", "amd64").expect("url");
    assert!(url.contains("/download/v0.1.4/"), "{url}");
    assert!(asset_url("latest", "linux", "amd64").is_err());
    assert!(asset_url("0.1.4", "windows", "amd64").is_err());
    assert!(asset_url("0.1.4", "linux", "x86_64").is_err());
    assert!(asset_url("0.1.4", "linux", "arm64").is_err());
    assert!(
        asset_url("0.1.4", "linux", "aarch64")
            .expect("aarch64")
            .contains("_aarch64.")
    );
}

#[test]
fn verify_file_sha256_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("empty");
    fs::write(&path, b"").expect("write");
    verify_file_sha256(&path, EMPTY_SHA256).expect("empty digest");
    assert!(verify_file_sha256(&path, "00".repeat(32).as_str()).is_err());
}

fn write_src(dir: &Path, bytes: &[u8]) -> std::path::PathBuf {
    let src = dir.join("payload");
    fs::write(&src, bytes).expect("write src");
    src
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).expect("meta").permissions().mode() & 0o777
}

#[test]
fn replace_file_atomic_creates_dest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = write_src(tmp.path(), b"new-bin");
    let dest = tmp.path().join("ggok");
    replace_file_atomic(&src, &dest).expect("replace");
    assert_eq!(fs::read(&dest).expect("read"), b"new-bin");
    assert_eq!(mode_of(&dest), 0o755);
}

#[test]
fn replace_file_atomic_replaces_and_drops_backup() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = write_src(tmp.path(), b"src-bytes");
    let dest = tmp.path().join("ggok");
    fs::write(&dest, b"old-bytes").expect("old dest");
    replace_file_atomic(&src, &dest).expect("replace");
    assert_eq!(fs::read(&dest).expect("read"), b"src-bytes");
    assert!(!tmp.path().join(".ggok.old").exists());
    assert!(!tmp.path().join(".ggok.new").exists());
}

#[test]
fn replace_file_atomic_creates_missing_parent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = write_src(tmp.path(), b"nested");
    let dest = tmp.path().join("missing").join("ggok");
    replace_file_atomic(&src, &dest).expect("replace");
    assert_eq!(fs::read(&dest).expect("read"), b"nested");
}

#[test]
fn replace_file_atomic_restores_interrupted_backup_then_replaces() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = write_src(tmp.path(), b"fresh");
    let dest = tmp.path().join("ggok");
    fs::write(tmp.path().join(".ggok.old"), b"rescued").expect("backup");
    replace_file_atomic(&src, &dest).expect("replace");
    assert_eq!(fs::read(&dest).expect("read"), b"fresh");
    assert!(!tmp.path().join(".ggok.old").exists());
}

#[test]
fn replace_file_atomic_copy_across_directories() {
    let src_dir = tempfile::tempdir().expect("src dir");
    let dest_dir = tempfile::tempdir().expect("dest dir");
    let src = write_src(src_dir.path(), b"cross-device");
    let dest = dest_dir.path().join("ggok");
    replace_file_atomic(&src, &dest).expect("copy not rename");
    assert_eq!(fs::read(&dest).expect("read"), b"cross-device");
    assert_eq!(fs::read(&src).expect("src kept"), b"cross-device");
}
