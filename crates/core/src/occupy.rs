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
    Tui,
    Disk,
}

impl Source {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::Observe => "observe",
            Self::Foreign => "foreign",
            Self::Tui => "tui",
            Self::Disk => "disk",
        }
    }

    /// Web may follow jsonl, but must not take the grok control plane.
    #[must_use]
    pub fn is_spectator(self) -> bool {
        matches!(self, Self::Observe | Self::Foreign | Self::Tui)
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
    pub can_attach: bool,
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
    let text = cmdline_haystack(cmd);
    text.contains("grok") && text.contains("--no-leader") && text.contains("stdio")
}

fn cmdline_haystack(cmd: &[u8]) -> String {
    String::from_utf8_lossy(cmd).replace('\0', " ")
}

#[must_use]
pub fn is_stdio_client_cmd(cmd: &[u8]) -> bool {
    let text = cmdline_haystack(cmd);
    text.contains("grok") && text.contains("stdio")
}

#[must_use]
pub fn is_leader_server_cmd(cmd: &[u8]) -> bool {
    let text = cmdline_haystack(cmd);
    text.contains("grok") && text.contains("leader") && !text.contains("stdio")
}

#[must_use]
pub fn is_auto_spawned_leader_cmd(cmd: &[u8]) -> bool {
    let text = cmdline_haystack(cmd);
    is_leader_server_cmd(cmd) && text.contains("--relay-on-demand")
}

#[must_use]
pub fn is_ggok_spawned_leader_cmd(cmd: &[u8]) -> bool {
    let text = cmdline_haystack(cmd);
    is_leader_server_cmd(cmd)
        && text.contains("--no-auto-update")
        && text.contains("--no-exit-on-disconnect")
}

#[must_use]
pub fn is_tui_cmd(cmd: &[u8]) -> bool {
    let text = cmdline_haystack(cmd);
    text.contains("grok") && !text.contains("stdio") && !is_leader_server_cmd(cmd)
}

#[must_use]
pub fn tui_held<S: ::std::hash::BuildHasher>(
    s3: &HashMap<String, u32, S>,
    id: &str,
    our_runtime_pid: Option<u32>,
) -> bool {
    let Some(&pid) = s3.get(id) else {
        return false;
    };
    if Some(pid) == our_runtime_pid {
        return false;
    }
    is_tui_cmd(&crate::sys::pid_cmdline(pid))
}

#[must_use]
pub fn should_cancel_web_peer(prev: &str, next: &str, prev_is_tui: bool) -> bool {
    prev != next && !prev_is_tui
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
    if let Some(&pid) = input.s3.get(input.id) {
        let cmd = crate::sys::pid_cmdline(pid);
        if s3_is_hard_foreign(pid, input.our_runtime_pid, input.can_attach, &cmd) {
            return Occupancy {
                source: peer_source(&cmd),
                writable: false,
                running: true,
            };
        }
    }
    if input.jsonl_running {
        if input.can_attach {
            return Occupancy {
                source: Source::Disk,
                writable: true,
                running: true,
            };
        }
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
pub fn s3_is_hard_foreign(
    holder: u32,
    our_runtime_pid: Option<u32>,
    can_attach: bool,
    cmd: &[u8],
) -> bool {
    if Some(holder) == our_runtime_pid {
        return false;
    }
    if is_noleader_stdio(cmd) || is_tui_cmd(cmd) {
        return true;
    }
    !can_attach
}

#[must_use]
pub fn peer_source(cmd: &[u8]) -> Source {
    if is_tui_cmd(cmd) {
        Source::Tui
    } else {
        Source::Foreign
    }
}

#[must_use]
pub fn conflict_busy(occ: Occupancy, op: SessionOp) -> bool {
    match op {
        SessionOp::Prompt => !occ.writable,
        SessionOp::Load | SessionOp::Cancel | SessionOp::Control => occ.source != Source::Attached,
        SessionOp::Delete => match occ.source {
            Source::Foreign | Source::Tui => true,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebActiveRecord {
    pub id: String,
}

#[must_use]
pub fn web_active_path(leader_json: &Path) -> PathBuf {
    leader_json
        .parent()
        .unwrap_or(Path::new("."))
        .join("web-active.json")
}

#[must_use]
pub fn read_web_active(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let rec: WebActiveRecord = serde_json::from_str(&raw).ok()?;
    uuid::Uuid::parse_str(rec.id.trim()).ok()?;
    Some(rec.id)
}

/// # Errors
/// Returns an error if `id` is not a UUID or the file cannot be written.
pub fn write_web_active(path: &Path, id: &str) -> std::io::Result<()> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid session id",
        ));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(&WebActiveRecord { id: id.to_string() })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderListEntry {
    pub classification: String,
    pub pid_live: Option<u32>,
}

#[must_use]
pub fn parse_leader_list(raw: &str) -> Vec<LeaderListEntry> {
    let Ok(v) = serde_json::from_str::<Value>(raw.trim()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    visit_leader_rows(&v, &mut |row| {
        let classification = row
            .get("classification")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let pid_live = json_pid(row.get("pidLive")).or_else(|| json_pid(row.get("pid_live")));
        out.push(LeaderListEntry {
            classification,
            pid_live,
        });
    });
    out
}

#[must_use]
pub fn first_reachable_leader_pid(raw: &str) -> Option<u32> {
    for row in parse_leader_list(raw) {
        if row.classification.eq_ignore_ascii_case("unreachable") {
            continue;
        }
        if let Some(pid) = row.pid_live.filter(|p| *p != 0) {
            return Some(pid);
        }
    }
    None
}

fn visit_leader_rows(v: &Value, visit: &mut dyn FnMut(&Value)) {
    if let Some(arr) = v.as_array() {
        for row in arr {
            visit(row);
        }
        return;
    }
    if let Some(arr) = v.get("leaders").and_then(Value::as_array) {
        for row in arr {
            visit(row);
        }
        return;
    }
    if let Some(arr) = v.get("items").and_then(Value::as_array) {
        for row in arr {
            visit(row);
        }
        return;
    }
    if v.is_object() {
        visit(v);
    }
}

fn json_pid(v: Option<&Value>) -> Option<u32> {
    let v = v?;
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .or_else(|| v.as_i64().and_then(|n| u32::try_from(n).ok()))
        .filter(|p| *p != 0)
}

#[must_use]
pub fn leader_is_independent(pid: u32) -> bool {
    let Some(parent) = crate::sys::pid_ppid(pid) else {
        return false;
    };
    if parent <= 1 {
        return true;
    }
    !is_stdio_client_cmd(&crate::sys::pid_cmdline(parent))
}

#[must_use]
pub fn stdio_holds_leader(stdio_pid: u32) -> bool {
    crate::sys::pid_children(stdio_pid)
        .into_iter()
        .any(|child| is_leader_server_cmd(&crate::sys::pid_cmdline(child)))
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
