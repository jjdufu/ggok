use ggok_core::release::{CURRENT_VERSION, fetch_latest_version, is_newer};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::watch;

const CACHE_OK: Duration = Duration::from_secs(60);
const CACHE_ERR: Duration = Duration::from_secs(15);

static CACHE: Mutex<Option<CacheEntry>> = Mutex::new(None);
static INFLIGHT: Mutex<Inflight> = Mutex::new(Inflight { rx: None });

struct CacheEntry {
    at: Instant,
    latest: Option<String>,
}

struct Inflight {
    rx: Option<watch::Receiver<Option<VersionView>>>,
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
    StaleOk,
    FreshErr,
    Miss,
}

/// Look-path snapshot: current version is always local and returned
/// immediately. A minute-fresh GitHub tag is included when cached;
/// otherwise a background fetch updates the next look and the payload
/// keeps the last known tag (or `None` if GitHub has not succeeded yet).
pub fn snapshot() -> VersionView {
    match cached() {
        Hit::FreshOk(ver) => version_view(CURRENT_VERSION, Some(&ver)),
        Hit::FreshErr => version_view(CURRENT_VERSION, None),
        Hit::StaleOk | Hit::Miss => {
            refresh();
            version_view(CURRENT_VERSION, last_ok().as_deref())
        }
    }
}

fn refresh() {
    tokio::spawn(async {
        let _ = load_shared().await;
    });
}

pub fn warm() {
    refresh();
}

async fn wait_shared(mut rx: watch::Receiver<Option<VersionView>>) -> Option<VersionView> {
    loop {
        if let Some(view) = rx.borrow().clone() {
            return Some(view);
        }
        if rx.changed().await.is_err() {
            return None;
        }
    }
}

async fn load_shared() -> VersionView {
    loop {
        let pending = INFLIGHT.lock().ok().and_then(|g| g.rx.clone());
        if let Some(rx) = pending {
            if let Some(view) = wait_shared(rx).await {
                return view;
            }
            continue;
        }

        let (tx, rx) = watch::channel(None);
        let existing = {
            let Ok(mut g) = INFLIGHT.lock() else {
                return load().await;
            };
            if let Some(existing) = g.rx.clone() {
                Some(existing)
            } else {
                g.rx = Some(rx);
                None
            }
        };
        if let Some(existing) = existing {
            if let Some(view) = wait_shared(existing).await {
                return view;
            }
            continue;
        }

        let view = load().await;
        let _ = tx.send(Some(view.clone()));
        if let Ok(mut g) = INFLIGHT.lock() {
            g.rx = None;
        }
        return view;
    }
}

async fn load() -> VersionView {
    let fetched = tokio::task::spawn_blocking(fetch_latest_version).await;
    match fetched {
        Ok(Ok(ver)) => {
            store(Some(ver.clone()));
            version_view(CURRENT_VERSION, Some(&ver))
        }
        Ok(Err(e)) => {
            tracing::warn!("version check failed: {e:#}");
            fallback_after_err()
        }
        Err(e) => {
            tracing::warn!("version check failed: {e:#}");
            fallback_after_err()
        }
    }
}

fn fallback_after_err() -> VersionView {
    if let Some(ver) = last_ok() {
        version_view(CURRENT_VERSION, Some(&ver))
    } else {
        store(None);
        version_view(CURRENT_VERSION, None)
    }
}

fn last_ok() -> Option<String> {
    let Ok(guard) = CACHE.lock() else {
        return None;
    };
    guard.as_ref().and_then(|entry| entry.latest.clone())
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
        Some(_) => Hit::StaleOk,
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
