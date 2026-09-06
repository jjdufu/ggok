use ggok_core::release::{CURRENT_VERSION, fetch_latest_version, is_newer};
use serde::Serialize;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const CACHE_OK: Duration = Duration::from_hours(6);
const CACHE_ERR: Duration = Duration::from_mins(15);

static CACHE: Mutex<Option<CacheEntry>> = Mutex::new(None);
static REFRESHING: AtomicBool = AtomicBool::new(false);

struct CacheEntry {
    at: Instant,
    latest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionView {
    pub version: String,
    pub latest: Option<String>,
    pub update_available: bool,
}

/// Build the `/api/version` payload. `latest` is `None` when GitHub has not
/// been checked yet or the last check failed.
#[must_use]
pub fn version_view(current: &str, latest: Option<&str>) -> VersionView {
    VersionView {
        version: current.to_string(),
        latest: latest.map(str::to_string),
        update_available: latest.is_some_and(|ver| is_newer(ver, current)),
    }
}

enum Hit {
    FreshOk(String),
    StaleOk(String),
    FreshErr,
    Miss,
}

#[must_use]
pub fn snapshot() -> VersionView {
    let hit = cached();
    match &hit {
        Hit::FreshOk(_) | Hit::FreshErr => {}
        Hit::StaleOk(_) | Hit::Miss => spawn_refresh(),
    }
    view_from(&hit)
}

pub fn warm() {
    match cached() {
        Hit::FreshOk(_) | Hit::StaleOk(_) => {}
        Hit::FreshErr | Hit::Miss => spawn_refresh(),
    }
}

fn view_from(hit: &Hit) -> VersionView {
    let latest = match hit {
        Hit::FreshOk(v) | Hit::StaleOk(v) => Some(v.as_str()),
        Hit::FreshErr | Hit::Miss => None,
    };
    version_view(CURRENT_VERSION, latest)
}

fn cached() -> Hit {
    let Ok(guard) = CACHE.lock() else {
        return Hit::Miss;
    };
    let Some(entry) = guard.as_ref() else {
        return Hit::Miss;
    };
    let age = entry.at.elapsed();
    match &entry.latest {
        Some(ver) if age <= CACHE_OK => Hit::FreshOk(ver.clone()),
        Some(ver) => Hit::StaleOk(ver.clone()),
        None if age <= CACHE_ERR => Hit::FreshErr,
        None => Hit::Miss,
    }
}

fn store(latest: Option<String>) {
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(CacheEntry {
            at: Instant::now(),
            latest,
        });
    }
}

fn spawn_refresh() {
    if REFRESHING.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async {
        let fetched = tokio::task::spawn_blocking(fetch_latest_version).await;
        match fetched {
            Ok(Ok(ver)) => store(Some(ver)),
            Ok(Err(e)) => {
                tracing::warn!("version check failed: {e:#}");
                store(None);
            }
            Err(e) => {
                tracing::warn!("version check failed: {e:#}");
                store(None);
            }
        }
        REFRESHING.store(false, Ordering::SeqCst);
    });
}
