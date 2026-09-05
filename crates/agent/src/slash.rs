use ggok_core::SlashCommand;
use std::collections::BTreeMap;

#[must_use]
pub fn merge_commands(docs: Vec<SlashCommand>, acp: Vec<SlashCommand>) -> Vec<SlashCommand> {
    let mut by_name: BTreeMap<String, SlashCommand> = BTreeMap::new();
    for cmd in docs {
        by_name.insert(cmd.name.clone(), cmd);
    }
    for cmd in acp {
        match by_name.get_mut(&cmd.name) {
            Some(existing) => {
                if existing.description.is_empty() && !cmd.description.is_empty() {
                    existing.description = cmd.description;
                }
                if existing.hint.is_none() {
                    existing.hint = cmd.hint;
                }
                for a in cmd.aliases {
                    if a != existing.name && !existing.aliases.iter().any(|x| x == &a) {
                        existing.aliases.push(a);
                    }
                }
            }
            None => {
                by_name.insert(cmd.name.clone(), cmd);
            }
        }
    }
    by_name.into_values().collect()
}
