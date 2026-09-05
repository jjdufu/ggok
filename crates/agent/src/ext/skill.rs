use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

const MAX_NAME: usize = 64;
const MAX_UNCOMPRESSED: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub label: String,
    pub description: String,
    pub path: String,
    pub scope: String,
    pub category: String,
}

#[must_use]
pub fn list(grok_home: &Path, cwd: Option<&Path>) -> Value {
    let mut skills = Vec::new();
    scan_dir(&grok_home.join("skills"), "user", &mut skills);
    if let Some(cwd) = cwd {
        scan_dir(&cwd.join(".grok").join("skills"), "project", &mut skills);
    }
    scan_dir(&grok_home.join("bundled").join("skills"), "bundled", &mut skills);
    json!({ "skills": skills })
}

fn scan_dir(root: &Path, scope: &str, out: &mut Vec<SkillInfo>) {
    if !root.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for ent in entries {
        let Ok(ent) = ent else { continue };
        let path = ent.path();
        if !path.is_dir() {
            continue;
        }
        let md = path.join("SKILL.md");
        if !md.is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&md) else {
            continue;
        };
        let meta = parse_frontmatter(&text);
        let name = meta
            .name
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| path.file_name().and_then(|s| s.to_str()).map(ToOwned::to_owned))
            .unwrap_or_else(|| "skill".to_string());
        let description = card_description(&meta);
        let label = skill_label(&name);
        let category = skill_category(scope, &name);
        out.push(SkillInfo {
            name,
            label,
            description,
            path: md.to_string_lossy().into_owned(),
            scope: scope.to_string(),
            category,
        });
    }
}

struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    short_description: Option<String>,
}

fn parse_frontmatter(text: &str) -> Frontmatter {
    let mut name = None;
    let mut description = None;
    let mut short_description = None;
    let trimmed = text.trim_start_matches('\u{feff}');
    let rest = trimmed.strip_prefix("---").or_else(|| trimmed.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return Frontmatter {
            name,
            description,
            short_description,
        };
    };
    let rest = rest.strip_prefix('\n').or_else(|| rest.strip_prefix("\r\n")).unwrap_or(rest);
    let end = rest.find("\n---").or_else(|| rest.find("\r\n---"));
    let block = end.map_or(rest, |i| &rest[..i]);
    for raw in block.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim();
        let val = unquote(v.trim());
        if key == "name" && name.is_none() {
            name = Some(val);
        } else if key == "description" && description.is_none() {
            description = Some(val);
        } else if (key == "short-description" || key == "short_description")
            && short_description.is_none()
        {
            short_description = Some(val);
        }
    }
    Frontmatter {
        name,
        description,
        short_description,
    }
}

fn looks_folded(raw: &str) -> bool {
    let s = raw.trim();
    s.is_empty() || s == ">" || s == ">-" || s == "|" || s == "|-"
}

fn card_description(meta: &Frontmatter) -> String {
    if let Some(s) = meta.short_description.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        return s.to_string();
    }
    let raw = meta.description.as_deref().unwrap_or("").trim();
    if looks_folded(raw) {
        return String::new();
    }
    let cut = raw.find(". ").map_or(raw.len(), |i| i + 1);
    let mut s = raw[..cut].trim().to_string();
    if s.chars().count() > 140 {
        s = s.chars().take(140).collect::<String>().trim().to_string();
        s.push('…');
    }
    s
}

fn skill_label(name: &str) -> String {
    match name {
        "docx" => "Word Documents".to_string(),
        "pdf" => "PDFs".to_string(),
        "pptx" => "Presentations".to_string(),
        "create-skill" => "Skill Creator".to_string(),
        _ => name
            .split('-')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let mut cs = p.chars();
                match cs.next() {
                    Some(c) => format!("{}{}", c.to_ascii_uppercase(), cs.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn skill_category(scope: &str, name: &str) -> String {
    if scope != "bundled" {
        return "personal".to_string();
    }
    if matches!(name, "docx" | "pdf" | "pptx") {
        return "documents".to_string();
    }
    if name.starts_with("game-") {
        return "game".to_string();
    }
    if name.starts_with("resume-") {
        return "resume".to_string();
    }
    "builtin".to_string()
}

fn unquote(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 2 {
        let b = s.as_bytes();
        if (b[0] == b'"' && b[s.len() - 1] == b'"') || (b[0] == b'\'' && b[s.len() - 1] == b'\'') {
            return s[1..s.len() - 1].replace("\\\"", "\"").replace("\\n", " ");
        }
    }
    s.to_string()
}

fn slug_name(raw: &str) -> Result<String> {
    let mut out = String::new();
    for c in raw.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if (c == '-' || c == '_' || c == ' ')
            && !out.ends_with('-')
            && !out.is_empty()
        {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        bail!("name required");
    }
    if out.len() > MAX_NAME {
        bail!("name must be at most {MAX_NAME} characters");
    }
    if !out
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!("name must be lowercase letters, numbers, and hyphens");
    }
    Ok(out)
}

fn yaml_escape(raw: &str) -> String {
    let flat: String = raw
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string();
    let needs = flat.is_empty()
        || flat.chars().any(|c| {
            matches!(
                c,
                ':' | '#' | '"' | '\'' | '{' | '}' | '[' | ']' | ',' | '&' | '*' | '!' | '|' | '>'
                    | '%' | '@' | '`'
            ) || c.is_whitespace() && c != ' '
        })
        || matches!(flat.as_bytes().first(), Some(b) if *b == b' ' || *b == b'-' || *b == b'?');
    if needs {
        format!("\"{}\"", flat.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        flat
    }
}

fn render_skill_md(name: &str, description: &str, body: &str) -> String {
    let body = body.trim();
    format!(
        "---\nname: {name}\ndescription: {}\n---\n\n{}\n",
        yaml_escape(description),
        body
    )
}

fn user_skill_dir(grok_home: &Path, name: &str) -> PathBuf {
    grok_home.join("skills").join(name)
}

fn write_skill_md(dir: &Path, contents: &str) -> Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join("SKILL.md");
    fs::write(&path, contents).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// # Errors
/// Returns an error if the name or description is invalid, or the skill file cannot be written.
pub fn create(grok_home: &Path, name: &str, description: &str, body: &str) -> Result<Value> {
    let slug = slug_name(name)?;
    let desc = description.trim();
    if desc.is_empty() {
        bail!("description required");
    }
    let dir = user_skill_dir(grok_home, &slug);
    let path = write_skill_md(&dir, &render_skill_md(&slug, desc, body))?;
    Ok(json!({
        "ok": true,
        "name": slug,
        "path": path.to_string_lossy(),
        "scope": "user",
    }))
}

/// # Errors
/// Returns an error if the upload is empty, too large, not a skill, or cannot be written.
pub fn upload(grok_home: &Path, filename: &str, bytes: &[u8]) -> Result<Value> {
    if bytes.is_empty() {
        bail!("file required");
    }
    if bytes.len() as u64 > MAX_UNCOMPRESSED {
        bail!("file too large");
    }
    let lower = filename.to_ascii_lowercase();
    let is_md = Path::new(&lower)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"));
    if is_zip_upload(&lower, bytes) {
        upload_zip(grok_home, bytes)
    } else if is_md {
        upload_markdown(grok_home, filename, bytes)
    } else {
        bail!("accept .md, .markdown, .zip, or .skill");
    }
}

fn is_zip_upload(lower_name: &str, bytes: &[u8]) -> bool {
    let magic = bytes.len() >= 4 && bytes[0] == b'P' && bytes[1] == b'K';
    let path = Path::new(lower_name);
    let is_zip = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
    let is_skill = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("skill"));
    let is_md = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
    is_zip || (is_skill && magic) || (magic && !is_md)
}

fn upload_markdown(grok_home: &Path, filename: &str, bytes: &[u8]) -> Result<Value> {
    let text = String::from_utf8(bytes.to_vec()).context("SKILL.md must be UTF-8")?;
    let meta = parse_frontmatter(&text);
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let stem = if stem.eq_ignore_ascii_case("skill") {
        ""
    } else {
        stem
    };
    let raw_name = meta
        .name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(stem);
    let slug = slug_name(raw_name)?;
    let desc = meta.description.unwrap_or_default();
    let dir = user_skill_dir(grok_home, &slug);
    let contents = if text.trim_start().starts_with("---") {
        text
    } else {
        render_skill_md(&slug, &desc, &text)
    };
    let path = write_skill_md(&dir, &contents)?;
    Ok(json!({
        "ok": true,
        "name": slug,
        "path": path.to_string_lossy(),
        "scope": "user",
    }))
}

fn safe_rel(name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    if name.ends_with('/') {
        return None;
    }
    let p = Path::new(&name);
    if p.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s == ".." || s.is_empty() {
                    return None;
                }
                out.push(s.as_ref());
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn find_skill_md(archive: &mut zip::ZipArchive<Cursor<&[u8]>>) -> Result<(usize, PathBuf)> {
    let mut found: Option<(usize, PathBuf)> = None;
    for i in 0..archive.len() {
        let file = archive.by_index(i).context("zip entry")?;
        let Some(rel) = safe_rel(file.name()) else {
            continue;
        };
        let fname = rel.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !fname.eq_ignore_ascii_case("SKILL.md") {
            continue;
        }
        let depth = rel.components().count();
        if depth == 1 {
            return Ok((i, rel));
        }
        if depth == 2 && found.is_none() {
            found = Some((i, rel));
        }
    }
    found.ok_or_else(|| anyhow::anyhow!("zip has no SKILL.md at the root or one folder deep"))
}

fn extract_zip_files(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    dest: &Path,
    prefix: Option<&Path>,
) -> Result<()> {
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("zip entry")?;
        if file.is_dir() {
            continue;
        }
        let Some(rel) = safe_rel(file.name()) else {
            continue;
        };
        let inner = if let Some(pref) = prefix {
            match rel.strip_prefix(pref) {
                Ok(p) if p.as_os_str().is_empty() => continue,
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            }
        } else {
            rel
        };
        if inner.components().any(|c| {
            matches!(c, Component::ParentDir)
                || c.as_os_str() == "__MACOSX"
                || c.as_os_str() == ".DS_Store"
        }) {
            continue;
        }
        total = total.saturating_add(file.size());
        if total > MAX_UNCOMPRESSED {
            bail!("zip too large");
        }
        let out_path = dest.join(&inner);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).context("extract zip file")?;
        fs::write(&out_path, buf).with_context(|| format!("write {}", out_path.display()))?;
    }
    Ok(())
}

fn upload_zip(grok_home: &Path, bytes: &[u8]) -> Result<Value> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("invalid zip")?;
    let (skill_idx, skill_rel) = find_skill_md(&mut archive)?;
    let md_bytes = {
        let mut file = archive.by_index(skill_idx).context("zip SKILL.md")?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).context("read SKILL.md")?;
        buf
    };
    if u64::try_from(md_bytes.len()).unwrap_or(u64::MAX) > MAX_UNCOMPRESSED {
        bail!("SKILL.md too large");
    }
    let text = String::from_utf8(md_bytes).context("SKILL.md must be UTF-8")?;
    let meta = parse_frontmatter(&text);
    let prefix = skill_rel
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from);
    let folder_name = prefix
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let raw_name = meta
        .name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(folder_name);
    let slug = slug_name(raw_name)?;
    let dest = user_skill_dir(grok_home, &slug);
    fs::create_dir_all(&dest).with_context(|| format!("create {}", dest.display()))?;
    extract_zip_files(&mut archive, &dest, prefix.as_deref())?;
    let path = dest.join("SKILL.md");
    if !path.is_file() {
        bail!("failed to extract SKILL.md");
    }
    Ok(json!({
        "ok": true,
        "name": slug,
        "path": path.to_string_lossy(),
        "scope": "user",
    }))
}

const MAX_TEXT: u64 = 512 * 1024;

/// # Errors
/// Returns an error if `name` is empty, the skill is missing, or the file cannot be read.
pub fn detail(grok_home: &Path, cwd: Option<&Path>, name: &str, scope: Option<&str>) -> Result<Value> {
    let raw = name.trim();
    if raw.is_empty() {
        bail!("name required");
    }
    let want_scope = scope.map(str::trim).filter(|s| !s.is_empty());
    let mut skills = Vec::new();
    scan_dir(&grok_home.join("skills"), "user", &mut skills);
    if let Some(cwd) = cwd {
        scan_dir(&cwd.join(".grok").join("skills"), "project", &mut skills);
    }
    scan_dir(&grok_home.join("bundled").join("skills"), "bundled", &mut skills);
    let found = skills.into_iter().find(|s| {
        if want_scope.is_some_and(|sc| s.scope != sc) {
            return false;
        }
        s.name.eq_ignore_ascii_case(raw)
    });
    let Some(found) = found else {
        bail!("skill not found");
    };
    let md = PathBuf::from(&found.path);
    let dir = md
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("invalid skill path"))?;
    let canon = fs::canonicalize(&dir).with_context(|| format!("stat {}", dir.display()))?;
    if !canon.is_dir() {
        bail!("skill directory missing");
    }
    let files = collect_files(&canon)?;
    let markdown = files.iter().find_map(|f| {
        let name = f.get("name").and_then(Value::as_str).unwrap_or("");
        if name.eq_ignore_ascii_case("SKILL.md") {
            f.get("text").and_then(Value::as_str).map(ToOwned::to_owned)
        } else {
            None
        }
    });
    Ok(json!({
        "name": found.name,
        "label": found.label,
        "description": found.description,
        "path": found.path,
        "scope": found.scope,
        "category": found.category,
        "markdown": markdown,
        "files": files,
    }))
}

fn collect_files(root: &Path) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    walk_files(root, root, &mut out, 0)?;
    out.sort_by(|a, b| {
        let an = a.get("name").and_then(Value::as_str).unwrap_or("");
        let bn = b.get("name").and_then(Value::as_str).unwrap_or("");
        skill_file_rank(an)
            .cmp(&skill_file_rank(bn))
            .then_with(|| an.to_lowercase().cmp(&bn.to_lowercase()))
    });
    Ok(out)
}

fn skill_file_rank(name: &str) -> u8 {
    if name.eq_ignore_ascii_case("SKILL.md") {
        0
    } else if name.to_ascii_lowercase().ends_with(".md") {
        1
    } else {
        2
    }
}

fn walk_files(root: &Path, dir: &Path, out: &mut Vec<Value>, depth: usize) -> Result<()> {
    if depth > 4 {
        return Ok(());
    }
    let rd = fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))?;
    for ent in rd.flatten() {
        let path = ent.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        let name = ent.file_name();
        if name == ".git" || name == "node_modules" || name == "__MACOSX" || name == ".DS_Store" {
            continue;
        }
        if meta.is_dir() {
            walk_files(root, &path, out, depth + 1)?;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if rel_s.is_empty() {
            continue;
        }
        let bytes = meta.len();
        let kind = file_kind(&rel_s);
        let text = if kind != "binary" && bytes > 0 && bytes <= MAX_TEXT {
            fs::read_to_string(&path).ok()
        } else {
            None
        };
        let kind = if text.is_none() && kind != "markdown" {
            if bytes > MAX_TEXT {
                "binary"
            } else {
                kind
            }
        } else {
            kind
        };
        out.push(json!({
            "name": rel_s,
            "kind": kind,
            "bytes": bytes,
            "text": text,
        }));
    }
    Ok(())
}

fn file_kind(rel: &str) -> &'static str {
    let lower = rel.to_ascii_lowercase();
    let ext = Path::new(&lower)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match ext {
        "md" | "markdown" => "markdown",
        "py" | "sh" | "bash" | "zsh" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "rb" | "pl"
        | "lua" | "r" | "go" | "rs" | "java" | "php" => "script",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "pdf" | "zip" | "gz" | "woff" | "woff2"
        | "bin" => "binary",
        _ => "text",
    }
}
