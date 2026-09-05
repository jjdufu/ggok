use crate::types::SlashCommand;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[must_use]
pub fn from_docs(grok_home: &Path) -> Vec<SlashCommand> {
    let path = grok_home.join("docs/user-guide/04-slash-commands.md");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    parse_docs(&text)
}

fn parse_docs(text: &str) -> Vec<SlashCommand> {
    let mut cmds: BTreeMap<String, SlashCommand> = BTreeMap::new();
    let mut pending: Vec<(String, Option<String>)> = Vec::new();
    let mut pending_aliases: Vec<String> = Vec::new();
    let mut pending_body: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("### ") {
            flush_pending(
                &mut pending,
                &mut pending_aliases,
                &mut pending_body,
                &mut cmds,
            );
            let heading = heading_commands(rest);
            pending = heading.commands;
            pending_aliases = heading.aliases;
            continue;
        }
        if !pending.is_empty() {
            pending_body.push(line.to_string());
        }
    }
    flush_pending(
        &mut pending,
        &mut pending_aliases,
        &mut pending_body,
        &mut cmds,
    );
    cmds.into_values().collect()
}

struct Heading {
    commands: Vec<(String, Option<String>)>,
    aliases: Vec<String>,
}

fn flush_pending(
    pending: &mut Vec<(String, Option<String>)>,
    pending_aliases: &mut Vec<String>,
    body: &mut Vec<String>,
    cmds: &mut BTreeMap<String, SlashCommand>,
) {
    if pending.is_empty() {
        body.clear();
        pending_aliases.clear();
        return;
    }
    let desc = first_paragraph(body);
    let mut extra_aliases = body_aliases(body);
    for a in pending_aliases.drain(..) {
        if !extra_aliases.iter().any(|x| x == &a) {
            extra_aliases.push(a);
        }
    }
    let heading = std::mem::take(pending);
    let names: Vec<String> = heading.iter().map(|(n, _)| n.clone()).collect();
    for (name, hint) in &heading {
        let mut aliases = extra_aliases.clone();
        for other in &names {
            if other != name && !aliases.iter().any(|x| x == other) {
                aliases.push(other.clone());
            }
        }
        aliases.retain(|a| a != name);
        let entry = cmds.entry(name.clone()).or_insert_with(|| SlashCommand {
            name: name.clone(),
            description: String::new(),
            hint: None,
            aliases: Vec::new(),
        });
        if entry.description.is_empty() && !desc.is_empty() {
            entry.description.clone_from(&desc);
        }
        if entry.hint.is_none() {
            entry.hint.clone_from(hint);
        }
        for a in aliases {
            if !entry.aliases.iter().any(|x| x == &a) {
                entry.aliases.push(a);
            }
        }
    }
    body.clear();
}

fn heading_commands(rest: &str) -> Heading {
    let ticks = extract_ticks(rest);
    if rest.to_ascii_lowercase().contains("alias") {
        let mut iter = ticks.into_iter();
        Heading {
            commands: iter.next().into_iter().collect(),
            aliases: iter.map(|(n, _)| n).collect(),
        }
    } else {
        Heading {
            commands: ticks,
            aliases: Vec::new(),
        }
    }
}

fn extract_ticks(rest: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = rest.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'`'
            && let Some(end) = rest[i + 1..].find('`')
        {
            let inner = &rest[i + 1..i + 1 + end];
            i = i + 2 + end;
            if let Some(cmd) = parse_tick(inner)
                && !out.iter().any(|(n, _)| n == &cmd.0)
            {
                out.push(cmd);
            }
            continue;
        }
        i += 1;
    }
    out
}

fn parse_tick(inner: &str) -> Option<(String, Option<String>)> {
    let inner = inner.trim();
    let stripped = inner.strip_prefix('/')?;
    let mut parts = stripped.split_whitespace();
    let name = parts.next()?.to_string();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let rest: Vec<&str> = parts.collect();
    let hint = if rest.is_empty() {
        None
    } else {
        let h = rest.join(" ");
        let h = h
            .trim_matches(|c| c == '<' || c == '>' || c == '[' || c == ']' || c == '|')
            .replace('\\', "");
        let h = h.trim();
        if h.is_empty() {
            None
        } else {
            Some(h.to_string())
        }
    };
    Some((name, hint))
}

fn first_paragraph(body: &[String]) -> String {
    let mut lines = Vec::new();
    let mut started = false;
    let mut in_code = false;
    for raw in body {
        let line = raw.trim();
        if line.starts_with("```") {
            in_code = !in_code;
            if started {
                break;
            }
            continue;
        }
        if in_code {
            continue;
        }
        if line.starts_with('|') || line.starts_with('#') {
            if started {
                break;
            }
            continue;
        }
        if line.is_empty() {
            if started {
                break;
            }
            continue;
        }
        started = true;
        lines.push(strip_md(line));
    }
    lines.join(" ")
}

fn body_aliases(body: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in body {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("alias:") || lower.contains("aliases:")) {
            continue;
        }
        for (name, _) in extract_ticks(line) {
            if !out.iter().any(|x| x == &name) {
                out.push(name);
            }
        }
    }
    out
}

fn strip_md(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut label = String::new();
            let mut ok = false;
            while let Some(&n) = chars.peek() {
                chars.next();
                if n == ']' {
                    ok = true;
                    break;
                }
                label.push(n);
            }
            if ok && chars.peek() == Some(&'(') {
                chars.next();
                for n in chars.by_ref() {
                    if n == ')' {
                        break;
                    }
                }
                out.push_str(&label);
            } else {
                out.push('[');
                out.push_str(&label);
                if ok {
                    out.push(']');
                }
            }
            continue;
        }
        if c == '*' || c == '`' {
            continue;
        }
        out.push(c);
    }
    out
}
