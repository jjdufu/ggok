use crate::auth::LoginLimiter;
use crate::http;
use anyhow::{Context, Result};
use ggok_agent::Agent;
use ggok_core::config::RuntimeConfig;
use ggok_core::parse::{ParsedSession, parse_updates_file};
use ggok_core::scan::{SessionIndex, scan};
use ggok_core::types::SessionMeta;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::signal;
use tracing::info;

const PARSE_CACHE_CAP: usize = 4;

pub(crate) struct AppState {
    pub(crate) token: String,
    pub(crate) cookie_key: [u8; 32],
    pub(crate) cookie_name: String,
    pub(crate) grok_home: PathBuf,
    pub(crate) grok_bin: PathBuf,
    pub(crate) sessions: RwLock<SessionIndex>,
    pub(crate) parse_cache: Mutex<ParseCache>,
    pub(crate) login_fails: Mutex<LoginLimiter>,
    pub(crate) agent: Agent,
    pub(crate) workspace_roots: Vec<PathBuf>,
    pub(crate) permission_mode: String,
    pub(crate) upload_max_bytes: u64,
    pub(crate) poll_secs: u64,
    pub(crate) agent_pid_file: PathBuf,
    pub(crate) pins_path: PathBuf,
}

pub(crate) struct ParseCache {
    entries: HashMap<String, CachedParse>,
    order: Vec<String>,
}

struct CachedParse {
    path: PathBuf,
    mtime: SystemTime,
    size: u64,
    parsed: Arc<ParsedSession>,
}

impl ParseCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn get(
        &mut self,
        id: &str,
        path: &Path,
        mtime: SystemTime,
        size: u64,
    ) -> Option<Arc<ParsedSession>> {
        let hit = self.entries.get(id).and_then(|e| {
            if e.path == path && e.mtime == mtime && e.size == size {
                Some(Arc::clone(&e.parsed))
            } else {
                None
            }
        });
        if hit.is_some() {
            self.touch(id);
        }
        hit
    }

    fn insert(
        &mut self,
        id: &str,
        path: PathBuf,
        mtime: SystemTime,
        size: u64,
        parsed: Arc<ParsedSession>,
    ) {
        if self.entries.len() >= PARSE_CACHE_CAP
            && !self.entries.contains_key(id)
            && let Some(old) = self.order.first().cloned()
        {
            self.entries.remove(&old);
            self.order.remove(0);
        }
        self.entries.insert(
            id.to_string(),
            CachedParse {
                path,
                mtime,
                size,
                parsed,
            },
        );
        self.touch(id);
    }

    fn touch(&mut self, id: &str) {
        self.order.retain(|x| x != id);
        self.order.push(id.to_string());
    }
}

impl AppState {
    pub fn new(cfg: &RuntimeConfig, sessions: SessionIndex) -> Arc<Self> {
        let agent = Agent::new(
            cfg.grok_bin.clone(),
            cfg.grok_home.clone(),
            cfg.permission_mode.clone(),
            cfg.agent_pid_file.clone(),
            cfg.leader_json_file.clone(),
        );
        Arc::new(Self {
            token: cfg.token.clone(),
            cookie_key: cfg.cookie_key,
            cookie_name: crate::auth::cookie_name_for_bind(&cfg.bind),
            grok_home: cfg.grok_home.clone(),
            grok_bin: cfg.grok_bin.clone(),
            sessions: RwLock::new(sessions),
            parse_cache: Mutex::new(ParseCache::new()),
            login_fails: Mutex::new(LoginLimiter::new()),
            agent,
            workspace_roots: cfg.workspace_roots.clone(),
            permission_mode: cfg.permission_mode.clone(),
            upload_max_bytes: cfg.upload_max_bytes,
            poll_secs: cfg.poll_secs,
            agent_pid_file: cfg.agent_pid_file.clone(),
            pins_path: ggok_core::session::pins_path_from_agent_pid(&cfg.agent_pid_file),
        })
    }

    pub fn replace_sessions(&self, next: SessionIndex) {
        *self.sessions.write() = next;
    }

    pub fn session(&self, id: &str) -> Option<SessionMeta> {
        self.sessions.read().get(id).cloned()
    }

    pub fn parsed(&self, meta: &SessionMeta) -> Result<Arc<ParsedSession>> {
        let path = meta.dir.join("updates.jsonl");
        let Ok(fs_meta) = std::fs::metadata(&path) else {
            return Ok(Arc::new(ParsedSession {
                blocks: Vec::new(),
                usage: ggok_core::types::TokenUsage::default(),
                context_tokens: 0,
                work_started_ms: None,
            }));
        };
        let mtime = fs_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let size = fs_meta.len();
        {
            let mut cache = self.parse_cache.lock();
            if let Some(cached) = cache.get(&meta.id, &path, mtime, size) {
                return Ok(cached);
            }
        }
        let parsed = Arc::new(parse_updates_file(&path)?);
        self.parse_cache
            .lock()
            .insert(&meta.id, path, mtime, size, Arc::clone(&parsed));
        Ok(parsed)
    }
}

pub struct Service {
    pub(crate) config: RuntimeConfig,
    pub(crate) state: Arc<AppState>,
}

impl Service {
    /// # Errors
    /// Returns an error if the grok sessions tree cannot be scanned.
    pub fn from_config(cfg: RuntimeConfig) -> Result<Arc<Self>> {
        let sessions = scan(&cfg.grok_home).context("scan grok sessions")?;
        info!(
            sessions = sessions.sessions.len(),
            grok_home = %cfg.grok_home.display(),
            "scanned sessions"
        );
        Ok(Self::new(cfg, sessions))
    }

    #[must_use]
    pub fn new(cfg: RuntimeConfig, sessions: SessionIndex) -> Arc<Self> {
        let state = AppState::new(&cfg, sessions);
        Arc::new(Self { config: cfg, state })
    }

    pub fn router(&self) -> axum::Router {
        http::router(self.state.clone())
    }

    /// # Errors
    /// Returns an error if `--bind` is invalid or the HTTP server fails.
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init();

        let addr: SocketAddr = self.config.bind.parse().context("invalid --bind address")?;
        let poll_secs = self.config.poll_secs;
        let grok_home = self.config.grok_home.clone();
        let poll_state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(poll_secs));
            interval.tick().await;
            loop {
                interval.tick().await;
                match scan(&grok_home) {
                    Ok(next) => {
                        let n = next.sessions.len();
                        poll_state.replace_sessions(next);
                        tracing::debug!(sessions = n, "refreshed session index");
                    }
                    Err(e) => tracing::warn!("session rescan failed: {e:#}"),
                }
            }
        });

        crate::account::warm(self.config.grok_home.clone());
        crate::release::warm();

        let boot = self.state.agent.clone();
        tokio::spawn(async move {
            if let Err(e) = boot.connect_existing_leader().await {
                tracing::info!("connect existing leader: {e:#}");
            }
        });

        let app = self.router();
        let make = app.into_make_service_with_connect_info::<SocketAddr>();
        info!(%addr, "listening");

        let agent = self.state.agent.clone();
        let result = axum_server::bind(addr)
            .handle(shutdown_handle())
            .serve(make)
            .await
            .context("http server");
        agent.shutdown().await;
        result
    }
}

fn shutdown_handle() -> axum_server::Handle {
    let handle = axum_server::Handle::new();
    let h = handle.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        h.shutdown();
    });
    handle
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };
    match signal::unix::signal(signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                () = ctrl_c => {}
                _ = term.recv() => {}
            }
        }
        Err(_) => ctrl_c.await,
    }
}
