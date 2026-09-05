use ggok_agent::slash::merge_commands;
use ggok_core::types::SlashCommand;

fn cmd(name: &str, desc: &str, hint: Option<&str>, aliases: &[&str]) -> SlashCommand {
    SlashCommand {
        name: name.into(),
        description: desc.into(),
        hint: hint.map(ToOwned::to_owned),
        aliases: aliases.iter().map(|s| (*s).to_string()).collect(),
    }
}

#[test]
fn merge_prefers_docs_and_fills_from_acp() {
    let docs = vec![cmd("help", "from docs", None, &["h"])];
    let acp = vec![
        cmd("help", "from acp", Some("[cmd]"), &["?"]),
        cmd("status", "acp only", None, &[]),
    ];
    let merged = merge_commands(docs, acp);
    let help = merged.iter().find(|c| c.name == "help").expect("help");
    assert_eq!(help.description, "from docs");
    assert_eq!(help.hint.as_deref(), Some("[cmd]"));
    assert!(help.aliases.iter().any(|a| a == "h"));
    assert!(help.aliases.iter().any(|a| a == "?"));
    assert!(merged.iter().any(|c| c.name == "status"));
}

#[test]
fn merge_fills_empty_docs_description() {
    let docs = vec![cmd("help", "", None, &[])];
    let acp = vec![cmd("help", "acp desc", None, &[])];
    let merged = merge_commands(docs, acp);
    assert_eq!(merged[0].description, "acp desc");
}
