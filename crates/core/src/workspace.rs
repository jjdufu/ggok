use crate::paths::{cwd_allowed, is_under};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, Seek, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub const LIST_CAP: usize = 2000;
pub const ARCHIVE_MAX_UNCOMPRESSED: u64 = 256 * 1024 * 1024;
pub const ARCHIVE_MAX_ENTRIES: usize = 8000;
pub const FILE_MAX_BYTES: u64 = 64 * 1024 * 1024;

const ARCHIVE_SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", "__pycache__"];

#[derive(Debug, Clone, Serialize)]
pub struct WsEntry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WsList {
    pub cwd: String,
    pub dir: String,
    pub abs: String,
    pub truncated: bool,
    pub entries: Vec<WsEntry>,
}

struct Resolved {
    cwd: PathBuf,
    joined: PathBuf,
    target: PathBuf,
    meta: fs::Metadata,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

fn rel_to(cwd: &Path, target: &Path) -> String {
    target
        .strip_prefix(cwd)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

fn normalize_rel(rel: &str) -> Result<String> {
    let rel = rel.trim();
    if rel.contains('\0') {
        bail!("invalid path");
    }
    if rel.starts_with('/') {
        bail!("absolute path not allowed");
    }
    Ok(rel.trim_end_matches('/').to_string())
}

fn resolve_inner(cwd: &str, rel: &str, roots: &[PathBuf], allow_empty: bool) -> Result<Resolved> {
    let cwd_canon = cwd_allowed(cwd, roots)?;
    let rel = normalize_rel(rel)?;
    if rel.is_empty() || rel == "." {
        if !allow_empty {
            bail!("path required");
        }
        let meta = fs::symlink_metadata(&cwd_canon)
            .with_context(|| format!("stat {}", cwd_canon.display()))?;
        return Ok(Resolved {
            cwd: cwd_canon.clone(),
            joined: cwd_canon.clone(),
            target: cwd_canon,
            meta,
        });
    }
    let joined = cwd_canon.join(&rel);
    let meta = match fs::symlink_metadata(&joined) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => bail!("not found"),
        Err(e) => {
            return Err(e).with_context(|| format!("stat {}", joined.display()));
        }
    };
    let target = match fs::canonicalize(&joined) {
        Ok(p) => p,
        Err(e) if e.kind() == io::ErrorKind::NotFound => bail!("not found"),
        Err(e) => {
            return Err(e).with_context(|| format!("canonicalize {}", joined.display()));
        }
    };
    if !is_under(&target, &cwd_canon) {
        bail!("path is outside cwd");
    }
    Ok(Resolved {
        cwd: cwd_canon,
        joined,
        target,
        meta,
    })
}

/// # Errors
/// Returns an error if the path cannot be resolved or is not a directory.
pub fn resolve_workspace_dir(cwd: &str, rel: &str, roots: &[PathBuf]) -> Result<PathBuf> {
    let resolved = resolve_inner(cwd, rel, roots, true)?;
    if !resolved.target.is_dir() {
        bail!("not a directory");
    }
    Ok(resolved.target)
}

/// # Errors
/// Returns an error if the path cannot be resolved inside the workspace.
pub fn resolve_workspace_entry(cwd: &str, rel: &str, roots: &[PathBuf]) -> Result<PathBuf> {
    Ok(resolve_inner(cwd, rel, roots, false)?.target)
}

fn delete_block_reason(target: &Path, cwd: &Path, roots: &[PathBuf]) -> Option<&'static str> {
    if target == cwd {
        return Some("cannot delete working directory");
    }
    if roots.iter().any(|r| r == target) {
        return Some("cannot delete workspace root");
    }
    if target == Path::new("/") {
        return Some("cannot delete home directory");
    }
    if let Ok(home) = fs::canonicalize(home_dir())
        && target == home.as_path()
    {
        return Some("cannot delete home directory");
    }
    None
}

/// # Errors
/// Returns an error if the directory cannot be resolved or read.
pub fn list_workspace(cwd: &str, rel: &str, roots: &[PathBuf]) -> Result<WsList> {
    let cwd_canon = cwd_allowed(cwd, roots)?;
    let dir_abs = resolve_workspace_dir(cwd, rel, roots)?;
    let dir_rel = rel_to(&cwd_canon, &dir_abs);
    let mut entries = Vec::new();
    let mut truncated = false;
    let rd = fs::read_dir(&dir_abs).with_context(|| format!("read {}", dir_abs.display()))?;
    for ent in rd.flatten() {
        if entries.len() >= LIST_CAP {
            truncated = true;
            break;
        }
        let name_os = ent.file_name();
        let Some(name) = name_os.to_str() else {
            continue;
        };
        if name == "." || name == ".." {
            continue;
        }
        let path_abs = ent.path();
        let Ok(meta) = fs::symlink_metadata(&path_abs) else {
            continue;
        };
        let ft = meta.file_type();
        let dir = if ft.is_symlink() {
            fs::metadata(&path_abs).is_ok_and(|m| m.is_dir())
        } else {
            ft.is_dir()
        };
        let size = if dir || ft.is_symlink() {
            0
        } else {
            meta.len()
        };
        let path = if dir_rel.is_empty() {
            name.to_string()
        } else {
            format!("{dir_rel}/{name}")
        };
        entries.push(WsEntry {
            name: name.to_string(),
            path,
            dir,
            size,
            mtime: meta.mtime(),
        });
    }
    entries.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(WsList {
        cwd: cwd_canon.to_string_lossy().into_owned(),
        dir: dir_rel,
        abs: dir_abs.to_string_lossy().into_owned(),
        truncated,
        entries,
    })
}

/// # Errors
/// Returns an error if the path is blocked, invalid, or cannot be removed.
pub fn delete_workspace(cwd: &str, rel: &str, roots: &[PathBuf]) -> Result<()> {
    let resolved = resolve_inner(cwd, rel, roots, false)?;
    if let Some(msg) = delete_block_reason(&resolved.target, &resolved.cwd, roots) {
        bail!("{msg}");
    }
    let ft = resolved.meta.file_type();
    if ft.is_symlink() || ft.is_file() {
        fs::remove_file(&resolved.joined)
            .with_context(|| format!("remove {}", resolved.joined.display()))?;
    } else if ft.is_dir() {
        fs::remove_dir_all(&resolved.joined)
            .with_context(|| format!("remove {}", resolved.joined.display()))?;
    } else {
        bail!("not a file or directory");
    }
    Ok(())
}

/// # Errors
/// Returns an error if the path is not a regular file under the workspace size limit.
pub fn open_workspace_file(cwd: &str, rel: &str, roots: &[PathBuf]) -> Result<(PathBuf, u64)> {
    let resolved = resolve_inner(cwd, rel, roots, false)?;
    if resolved.meta.file_type().is_symlink() {
        bail!("symlink not allowed");
    }
    if !resolved.meta.file_type().is_file() {
        bail!("not a file");
    }
    let len = resolved.meta.len();
    if len > FILE_MAX_BYTES {
        bail!("file too large");
    }
    Ok((resolved.target, len))
}

fn zip_name(rel: &str, is_dir: bool) -> Result<String> {
    if rel.is_empty() {
        bail!("invalid zip path");
    }
    if rel.starts_with('/') || rel.contains('\0') {
        bail!("invalid zip path");
    }
    let mut out = String::new();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            bail!("invalid zip path");
        }
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(seg);
    }
    if is_dir && !out.ends_with('/') {
        out.push('/');
    }
    Ok(out)
}

struct WalkItem {
    abs: PathBuf,
    zip_rel: String,
}

/// # Errors
/// Returns an error if the directory cannot be resolved, is too large, or the zip cannot be written.
pub fn write_archive(
    cwd: &str,
    rel: &str,
    roots: &[PathBuf],
    out: impl Write + Seek,
) -> Result<()> {
    let root = resolve_workspace_dir(cwd, rel, roots)?;
    let opts_file = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let opts_dir = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);
    let mut zip = ZipWriter::new(out);
    let mut stack = vec![WalkItem {
        abs: root,
        zip_rel: String::new(),
    }];
    let mut entries = 0usize;
    let mut uncompressed = 0u64;
    while let Some(item) = stack.pop() {
        let rd = fs::read_dir(&item.abs).with_context(|| format!("read {}", item.abs.display()))?;
        for ent in rd.flatten() {
            let name_os = ent.file_name();
            let Some(name) = name_os.to_str() else {
                continue;
            };
            if name == "." || name == ".." {
                continue;
            }
            let abs = ent.path();
            let Ok(meta) = fs::symlink_metadata(&abs) else {
                continue;
            };
            let ft = meta.file_type();
            if ft.is_symlink() {
                continue;
            }
            let child_rel = if item.zip_rel.is_empty() {
                name.to_string()
            } else {
                format!("{}/{name}", item.zip_rel)
            };
            if ft.is_dir() {
                if ARCHIVE_SKIP_DIRS.contains(&name) {
                    continue;
                }
                entries += 1;
                if entries > ARCHIVE_MAX_ENTRIES {
                    bail!("too many files");
                }
                let zip_path = zip_name(&child_rel, true)?;
                zip.add_directory(&zip_path, opts_dir)?;
                stack.push(WalkItem {
                    abs,
                    zip_rel: child_rel,
                });
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let len = meta.len();
            if len > FILE_MAX_BYTES {
                bail!("file too large");
            }
            uncompressed = uncompressed.saturating_add(len);
            if uncompressed > ARCHIVE_MAX_UNCOMPRESSED {
                bail!("archive too large");
            }
            entries += 1;
            if entries > ARCHIVE_MAX_ENTRIES {
                bail!("too many files");
            }
            let zip_path = zip_name(&child_rel, false)?;
            zip.start_file(&zip_path, opts_file)?;
            let mut file = File::open(&abs).with_context(|| format!("open {}", abs.display()))?;
            io::copy(&mut file, &mut zip)?;
        }
    }
    zip.finish()?;
    Ok(())
}
