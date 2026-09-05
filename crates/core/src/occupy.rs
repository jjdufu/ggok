use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Agent,
    Cli,
    Disk,
}

impl Source {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Cli => "cli",
            Self::Disk => "disk",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Occupancy {
    pub source: Source,
    pub writable: bool,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct LiveView {
    pub loaded: bool,
    pub running: bool,
    pub model: String,
    pub effort: String,
}

#[derive(Debug, Deserialize)]
struct ActiveRow {
    session_id: String,
    pid: u32,
}

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
pub fn cli_sessions(grok_home: &Path, agent_pid: Option<u32>) -> HashMap<String, u32> {
    let path = grok_home.join("active_sessions.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let rows: Vec<ActiveRow> = if let Ok(list) = serde_json::from_str(&raw) {
        list
    } else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for row in rows {
        if Some(row.pid) == agent_pid {
            continue;
        }
        if !pid_alive(row.pid) {
            continue;
        }
        out.insert(row.session_id, row.pid);
    }
    out
}

#[must_use]
pub fn classify<S: ::std::hash::BuildHasher>(
    id: &str,
    live: Option<&LiveView>,
    cli: &HashMap<String, u32, S>,
) -> Occupancy {
    if live.is_some_and(|l| l.loaded) {
        let running = live.is_some_and(|l| l.running);
        return Occupancy {
            source: Source::Agent,
            writable: true,
            running,
        };
    }
    if cli.contains_key(id) {
        return Occupancy {
            source: Source::Cli,
            writable: true,
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
pub fn agent_pid(pid_file: &Path, live_pid: Option<u32>) -> Option<u32> {
    live_pid.or_else(|| read_pid_file(pid_file))
}
