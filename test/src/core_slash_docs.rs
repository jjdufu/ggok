use ggok_core::slash_docs::from_docs;
use std::fs;

#[test]
fn from_docs_missing_file_is_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(from_docs(dir.path()).is_empty());
}

#[test]
fn from_docs_parses_headings_aliases_and_body() {
    let dir = tempfile::tempdir().expect("tempdir");
    let docs = dir.path().join("docs/user-guide");
    fs::create_dir_all(&docs).expect("mkdir");
    fs::write(
        docs.join("04-slash-commands.md"),
        r#"
### `/help` `/h` (alias)

Show available commands.

### `/model <name>`

Switch the model.

Ignored text without a command heading.
"#,
    )
    .expect("write docs");

    let cmds = from_docs(dir.path());
    let help = cmds.iter().find(|c| c.name == "help").expect("help");
    assert!(help.aliases.iter().any(|a| a == "h"));
    assert!(help.description.contains("available commands"));

    let model = cmds.iter().find(|c| c.name == "model").expect("model");
    assert_eq!(model.hint.as_deref(), Some("name"));
}
