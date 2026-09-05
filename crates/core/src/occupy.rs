use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const SESSION_BUSY: &str = "session_busy";

const JSONL_WINDOW: u64 = 64 * 1024;
const RUNNING_AGE: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Attached,
    Observe,
    Foreign,
    Disk,
}

impl Source {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Observe => "observe",
            Self::Foreign => "foreign",
            Self::Disk => "disk",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupancy {
    pub source: Source,
    pub writable: bool,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOp {
    Prompt,
    Load,
    Cancel,
    Control,
    Delete,
}

#[derive(Debug, Clone)]
pub struct LiveView {
    pub loaded: bool,
    pub running: bool,
    pub model: String,
    pub effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaderRecord {
    pub pid: u32,
    pub owned: bool,
    pub socket: String,
}

pub struct ClassifyInput<'a> {
    pub id: &'a str,
    pub live: Option<&'a LiveView>,
    pub our_runtime_pid: Option<u32>,
    pub s3: &'a HashMap<String, u32>,
    pub leftover_noleader_alive: bool,
    pub jsonl_running: bool,
}

#[derive(Debug, Deserialize)]
struct ActiveRow {
    session_id: String,
    pid: u32,
}

struct JsonlKey {
    mtime_ns: u128,
    size: u64,
    running: bool,
}

static JSONL_CACHE: LazyLock<Mutex<HashMap<PathBuf, JsonlKey>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[must_use]
pub fn pid_alive(pid: u32) -> bool {
    crate::sys::pid_is_alive(pid)
}

#[must_use]
pub fn read_pid_file(path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse().ok()
}

#[must_use]
pub fn cmdline_matches_grok(cmd: &[u8]) -> bool {
    let text = String::from_utf8_lossy(cmd);
    if !text.contains("grok") {
        return false;
    }
    if text.contains("stdio") || text.contains("leader") {
        return true;
    }
    argv0_is_grok(cmd) && !text.contains("stdio")
}

#[must_use]
pub fn is_noleader_stdio(cmd: &[u8]) -> bool {
    let text = String::from_utf8_lossy(cmd);
    text.contains("grok") && text.contains("--no-leader") && text.contains("stdio")
}

fn argv0_is_grok(cmd: &[u8]) -> bool {
    let argv0 = cmd.split(|b| *b == 0).next().unwrap_or(cmd);
    let argv0 = String::from_utf8_lossy(argv0);
    Path::new(argv0.as_ref())
        .file_name()
        .and_then(|n| n.to_str())
        == Some("grok")
}

#[must_use]
pub fn cli_sessions(grok_home: &Path) -> HashMap<String, u32> {
    let path = grok_home.join("active_sessions.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(rows) = serde_json::from_str::<Vec<ActiveRow>>(&raw) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for row in rows {
        if !pid_alive(row.pid) {
            continue;
        }
        let cmd = crate::sys::pid_cmdline(row.pid);
        if !cmdline_matches_grok(&cmd) {
            continue;
        }
        out.insert(row.session_id, row.pid);
    }
    out
}

#[must_use]
pub fn leftover_noleader_pid(pid_file: &Path) -> Option<u32> {
    let pid = read_pid_file(pid_file)?;
    if !pid_alive(pid) {
        let _ = fs::remove_file(pid_file);
        return None;
    }
    let cmd = crate::sys::pid_cmdline(pid);
    if is_noleader_stdio(&cmd) {
        Some(pid)
    } else {
        let _ = fs::remove_file(pid_file);
        None
    }
}

#[must_use]
pub fn leftover_idle(grok_home: &Path) -> bool {
    let Ok(index) = crate::scan::scan(grok_home) else {
        return true;
    };
    let s3 = cli_sessions(grok_home);
    for (id, meta) in &index.sessions {
        if !jsonl_running(&meta.dir) {
            continue;
        }
        if s3.contains_key(id) {
            continue;
        }
        return false;
    }
    true
}

pub fn terminate_pid(pid: u32, sig: &str) {
    let _ = std::process::Command::new("kill")
        .arg(format!("-{sig}"))
        .arg(pid.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

pub fn reap_idle_leftover(pid_file: &Path, grok_home: &Path) {
    let Some(pid) = leftover_noleader_pid(pid_file) else {
        return;
    };
    if !leftover_idle(grok_home) {
        return;
    }
    terminate_pid(pid, "TERM");
    std::thread::sleep(Duration::from_millis(200));
    if pid_alive(pid) {
        terminate_pid(pid, "KILL");
    }
    let _ = fs::remove_file(pid_file);
}

#[must_use]
pub fn classify(input: &ClassifyInput<'_>) -> Occupancy {
    if input.live.is_some_and(|l| l.loaded) {
        return Occupancy {
            source: Source::Attached,
            writable: true,
            running: input.live.is_some_and(|l| l.running),
        };
    }
    if input.leftover_noleader_alive {
        return Occupancy {
            source: Source::Observe,
            writable: false,
            running: input.jsonl_running,
        };
    }
    if let Some(&pid) = input.s3.get(input.id)
        && Some(pid) != input.our_runtime_pid
    {
        return Occupancy {
            source: Source::Foreign,
            writable: false,
            running: true,
        };
    }
    if input.jsonl_running {
        return Occupancy {
            source: Source::Observe,
            writable: false,
            running: true,
        };
    }
    Occupancy {
        source: Source::Disk,
        writable: true,
        running: false,
    }
}

#[must_use]
pub fn conflict_busy(occ: Occupancy, op: SessionOp) -> bool {
    match op {
        SessionOp::Prompt => !occ.writable,
        SessionOp::Load | SessionOp::Cancel | SessionOp::Control => occ.source != Source::Attached,
        SessionOp::Delete => match occ.source {
            Source::Foreign => true,
            Source::Observe => occ.running,
            Source::Attached | Source::Disk => false,
        },
    }
}

#[must_use]
pub fn jsonl_running(session_dir: &Path) -> bool {
    let path = session_dir.join("updates.jsonl");
    let Ok(meta) = fs::metadata(&path) else {
        return false;
    };
    let mtime = meta.modified().unwrap_or(UNIX_EPOCH);
    let size = meta.len();
    let mtime_ns = mtime.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    {
        let cache = JSONL_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(hit) = cache.get(&path)
            && hit.mtime_ns == mtime_ns
            && hit.size == size
        {
            return hit.running;
        }
    }
    let running = jsonl_running_uncached(&path, mtime, size);
    let mut cache = JSONL_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.insert(
        path,
        JsonlKey {
            mtime_ns,
            size,
            running,
        },
    );
    running
}

fn jsonl_running_uncached(path: &Path, mtime: SystemTime, size: u64) -> bool {
    let last = last_session_update(path, size);
    if last.as_deref() == Some("turn_completed") {
        return false;
    }
    let age = SystemTime::now()
        .duration_since(mtime)
        .unwrap_or(Duration::MAX);
    age <= RUNNING_AGE
}

fn last_session_update(path: &Path, size: u64) -> Option<String> {
    let start = size.saturating_sub(JSONL_WINDOW);
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    let body = if start > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        text.as_ref()
    };
    let mut last = None;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(kind) = v
            .pointer("/params/update/sessionUpdate")
            .and_then(Value::as_str)
        {
            last = Some(kind.to_string());
        }
    }
    last
}

#[must_use]
pub fn read_leader_record(path: &Path) -> Option<LeaderRecord> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// # Errors
/// Returns an error if the parent directory or the file cannot be written.
pub fn write_leader_record(path: &Path, rec: &LeaderRecord) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(rec)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

#[must_use]
pub fn our_runtime_pid(live_pid: Option<u32>) -> Option<u32> {
    let pid = live_pid?;
    if !pid_alive(pid) {
        return None;
    }
    let cmd = crate::sys::pid_cmdline(pid);
    cmdline_matches_grok(&cmd).then_some(pid)
}
