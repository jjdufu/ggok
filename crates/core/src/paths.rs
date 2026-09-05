use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const UPLOAD_DIR: &str = "/tmp/.ggok-uploads";

#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub git: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub dir: bool,
}

#[must_use]
pub fn expand_roots(raw: Option<&[String]>) -> Vec<PathBuf> {
    let items: &[String] = match raw {
        None => &[],
        Some(v) => v,
    };
    let mut out = Vec::new();
    for item in items {
        if let Some(p) = expand_one(item)
            && !out.iter().any(|x: &PathBuf| x == &p)
        {
            out.push(p);
        }
    }
    out
}

fn expand_one(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let expanded = expand_tilde(trimmed);
    if !expanded.is_absolute() {
        return None;
    }
    let canon = fs::canonicalize(&expanded).ok()?;
    if canon.is_dir() { Some(canon) } else { None }
}

fn expand_tilde(raw: &str) -> PathBuf {
    if raw == "~" {
        return home_dir();
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(raw)
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// # Errors
/// Returns an error if `raw` is empty, not absolute, missing, or not a directory.
pub fn resolve_existing_dir(raw: &str) -> Result<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("cwd is empty");
    }
    let p = expand_tilde(trimmed);
    if !p.is_absolute() {
        bail!("cwd must be an absolute path");
    }
    let canon = fs::canonicalize(&p).with_context(|| format!("cwd {}", p.display()))?;
    if !canon.is_dir() {
        bail!("cwd is not a directory");
    }
    Ok(canon)
}

/// # Errors
/// Returns an error if the directory cannot be resolved or lies outside `roots`.
pub fn cwd_allowed(raw: &str, roots: &[PathBuf]) -> Result<PathBuf> {
    let canon = resolve_existing_dir(raw)?;
    if under_any_root(&canon, roots) {
        Ok(canon)
    } else {
        bail!("cwd is outside workspace_roots");
    }
}

#[must_use]
pub fn is_under(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

#[must_use]
pub fn under_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    if roots.is_empty() {
        return path.is_absolute();
    }
    roots.iter().any(|r| is_under(path, r))
}

/// # Errors
/// Returns an error if the parent cannot be resolved, is outside `roots`, or cannot be read.
pub fn list_dirs(parent: Option<&str>, roots: &[PathBuf]) -> Result<Vec<DirEntry>> {
    if let Some(parent) = parent.filter(|s| !s.is_empty()) {
        let dir = resolve_existing_dir(parent)?;
        if !under_any_root(&dir, roots) {
            bail!("parent is outside workspace_roots");
        }
        return read_child_dirs(&dir, roots);
    }
    if roots.is_empty() {
        let home = fs::canonicalize(home_dir()).unwrap_or_else(|_| home_dir());
        return Ok(vec![root_entry(&home)]);
    }
    Ok(roots.iter().map(|p| root_entry(p)).collect())
}

fn root_entry(path: &Path) -> DirEntry {
    let full = path.to_string_lossy().into_owned();
    DirEntry {
        name: full.clone(),
        path: full,
        git: path.join(".git").exists(),
    }
}

fn read_child_dirs(dir: &Path, roots: &[PathBuf]) -> Result<Vec<DirEntry>> {
    let mut out = Vec::new();
    let rd = fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))?;
    for ent in rd.flatten() {
        let name = ent.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == "." || name == ".." || name == ".git" {
            continue;
        }
        let Ok(canon) = fs::canonicalize(ent.path()) else {
            continue;
        };
        if !canon.is_dir() || !under_any_root(&canon, roots) {
            continue;
        }
        out.push(DirEntry {
            name: name.to_string(),
            path: canon.to_string_lossy().into_owned(),
            git: canon.join(".git").exists(),
        });
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

/// # Errors
/// Returns an error if `cwd` cannot be resolved.
pub fn fs_complete(cwd: &str, q: &str) -> Result<Vec<FsEntry>> {
    const SCAN_CAP: usize = 8000;
    const OUT_CAP: usize = 40;

    let cwd = resolve_existing_dir(cwd)?;
    let raw = q.trim();
    let bang = raw.starts_with('!');
    let rest = if bang { raw[1..].trim() } else { raw };
    let rest = strip_line_range(rest);
    let browse = rest.ends_with('/');
    let query = if browse {
        rest.trim_end_matches('/')
    } else {
        rest
    };

    let mut walk_root = cwd.clone();
    let mut max_depth = None;
    let mut needle = query;
    if browse {
        let dir = if query.is_empty() {
            cwd.clone()
        } else {
            let p = cwd.join(query);
            fs::canonicalize(&p).unwrap_or(p)
        };
        if !dir.is_dir() || !is_under(&dir, &cwd) {
            return Ok(Vec::new());
        }
        walk_root = dir;
        max_depth = Some(1);
        needle = "";
    }

    let mut wb = ignore::WalkBuilder::new(&walk_root);
    wb.follow_links(false);
    wb.hidden(!bang);
    wb.git_ignore(!bang);
    wb.git_global(!bang);
    wb.git_exclude(!bang);
    wb.ignore(!bang);
    if let Some(d) = max_depth {
        wb.max_depth(Some(d));
    }
    wb.filter_entry(|e| e.file_name() != ".git");

    let mut scored: Vec<(i64, FsEntry)> = Vec::new();
    let mut scanned = 0usize;
    for ent in wb.build().flatten() {
        let path = ent.path();
        if path == walk_root || path == cwd {
            continue;
        }
        scanned += 1;
        if scanned > SCAN_CAP {
            break;
        }
        let Ok(rel_path) = path.strip_prefix(&cwd) else {
            continue;
        };
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        if rel.is_empty() || rel == "." {
            continue;
        }
        let is_dir = ent.file_type().is_some_and(|t| t.is_dir());
        let Some(score) = fuzzy_score(&rel, needle) else {
            continue;
        };
        scored.push((
            score,
            FsEntry {
                name: path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&rel)
                    .to_string(),
                path: rel.clone(),
                dir: is_dir,
            },
        ));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.to_lowercase().cmp(&b.1.path.to_lowercase())));
    scored.truncate(OUT_CAP);
    Ok(scored.into_iter().map(|(_, e)| e).collect())
}

fn strip_line_range(q: &str) -> &str {
    if let Some(i) = q.rfind(':') {
        let rest = &q[i + 1..];
        if !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'-')
        {
            return &q[..i];
        }
    }
    q
}

fn fuzzy_score(rel: &str, query: &str) -> Option<i64> {
    let rel_l = rel.to_lowercase();
    let q = query.to_lowercase();
    let depth = i64::try_from(rel.bytes().filter(|b| *b == b'/').count()).unwrap_or(0);
    let rel_len = i64::try_from(rel.len()).unwrap_or(0);
    if q.is_empty() {
        return Some(10_000 - depth * 30 - rel_len);
    }
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let name_l = name.to_lowercase();
    let name_len = i64::try_from(name.len()).unwrap_or(0);
    if name_l == q {
        return Some(2_000_000 - depth);
    }
    if name_l.starts_with(&q) {
        return Some(1_000_000 - name_len);
    }
    if rel_l.starts_with(&q) {
        return Some(800_000 - rel_len);
    }
    if name_l.contains(&q) {
        return Some(500_000 - name_len);
    }
    if rel_l.contains(&q) {
        return Some(200_000 - rel_len);
    }
    let mut it = rel_l.chars();
    for c in q.chars() {
        loop {
            match it.next() {
                Some(x) if x == c => break,
                Some(_) => {}
                None => return None,
            }
        }
    }
    Some(50_000 - rel_len - depth * 10)
}

/// # Errors
/// Returns an error if the path is not a file under the upload directory.
pub fn open_upload(raw: &str) -> Result<PathBuf> {
    let given = PathBuf::from(raw);
    let path = if given.is_absolute() {
        given.canonicalize()?
    } else {
        PathBuf::from(UPLOAD_DIR).join(given).canonicalize()?
    };
    let root = PathBuf::from(UPLOAD_DIR)
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("not an upload"))?;
    if !path.starts_with(&root) {
        bail!("not an upload");
    }
    if !path.is_file() {
        bail!("not a file");
    }
    Ok(path)
}

#[must_use]
pub fn compress_upload(filename: &str, bytes: Vec<u8>) -> Vec<u8> {
    let lower = filename.to_ascii_lowercase();
    let png = Path::new(&lower)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        || mime_guess::from_path(filename)
            .first_or_octet_stream()
            .essence_str()
            == "image/png";
    if !png {
        return bytes;
    }
    let mut opts = oxipng::Options::from_preset(2);
    opts.strip = oxipng::StripChunks::Safe;
    opts.timeout = Some(std::time::Duration::from_secs(8));
    opts.optimize_alpha = false;
    opts.scale_16 = false;
    match oxipng::optimize_from_memory(&bytes, &opts) {
        Ok(out) if !out.is_empty() && out.len() <= bytes.len() => out,
        _ => bytes,
    }
}

/// # Errors
/// Returns an error if the file is too large, the name is invalid, or the write fails.
pub fn save_upload(filename: &str, bytes: &[u8], max: u64) -> Result<PathBuf> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max {
        bail!("file exceeds upload_max_bytes");
    }
    let name = safe_filename(filename)?;
    let dir = upload_dir()?;
    let dest = unique_path(&dir, &name);
    fs::write(&dest, bytes).with_context(|| format!("write {}", dest.display()))?;
    Ok(dest)
}

fn upload_dir() -> Result<PathBuf> {
    let dir = PathBuf::from(UPLOAD_DIR);
    ensure_upload_dir(&dir)?;
    Ok(dir)
}

fn ensure_upload_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let meta = fs::metadata(dir).with_context(|| format!("stat {}", dir.display()))?;
    if !meta.is_dir() {
        bail!("{} is not a directory", dir.display());
    }
    if let Some(uid) = effective_uid()
        && meta.uid() != uid
    {
        bail!("{} is owned by another user", dir.display());
    }
    let mut perms = meta.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(dir, perms).with_context(|| format!("chmod 0700 {}", dir.display()))?;
    Ok(())
}

fn effective_uid() -> Option<u32> {
    crate::sys::effective_uid()
}

fn safe_filename(name: &str) -> Result<String> {
    let base = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();
    if base.is_empty() || base == "." || base == ".." {
        bail!("invalid file name");
    }
    if base.contains('/') || base.contains('\\') {
        bail!("invalid file name");
    }
    Ok(base.to_string())
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let dest = dir.join(name);
    if !dest.exists() {
        return dest;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    for i in 1..1000 {
        let candidate = dir.join(format!("{stem}-{i}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-dup{ext}"))
}
