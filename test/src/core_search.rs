use ggok_core::search::search_session_ids;
use std::fs;

#[test]
fn search_empty_or_missing_db() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(search_session_ids(dir.path(), "").is_none());
    assert!(search_session_ids(dir.path(), "   ").is_none());
    assert!(search_session_ids(dir.path(), "hello").is_none());

    fs::create_dir_all(dir.path().join("sessions")).expect("mkdir");
    fs::write(
        dir.path().join("sessions/session_search.sqlite"),
        b"not sqlite",
    )
    .expect("write");
    assert!(search_session_ids(dir.path(), "hello").is_none());
}
