use std::fs;
use std::path::PathBuf;

fn cli_update_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("crates/cli/src/update.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn update_prints_steps_after_download_bar() {
    let src = cli_update_src();
    let run = src.find("pub(crate) fn run()").expect("run");
    let verify_fn = src.find("fn curl_and_verify").expect("curl_and_verify");
    let download = src[verify_fn..].find("curl_download").expect("download");
    let verify = src[verify_fn..]
        .find("println!(\"Verifying.\")")
        .expect("verify");
    assert!(
        download < verify,
        "Verifying must follow the download bar:\n{}",
        &src[verify_fn..verify_fn + verify + 24]
    );
    let call = src[run..verify_fn]
        .find("curl_and_verify(")
        .expect("call verify");
    let install = src[run..verify_fn]
        .find("println!(\"Installing.\")")
        .expect("install");
    let done = src[run..verify_fn]
        .find("println!(\"Updated to {latest}.\")")
        .expect("done");
    assert!(
        call < install && install < done,
        "install sits between verify and Updated to"
    );
}
