pub mod ext;
pub mod mcp_ask;
pub(crate) mod process;
pub(crate) mod question;
pub(crate) mod rpc;
pub(crate) mod session;
pub mod session_ops;
pub mod slash;
pub mod tail;

pub use mcp_ask::run_mcp_ask;
pub use question::{AskBridge, AskOption, AskQuestion, QuestionReply, QuestionView};

use ggok_core::occupy::{self, LiveView};
use ggok_core::parse::Parser;
use ggok_core::types::{Block, ModelInfo, QueueItem, SlashCommand, TokenUsage};
use question::PendingQuestion;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::{Child, ChildStdin};
use tokio::sync::{Mutex, broadcast, oneshot};

#[derive(Clone)]
pub struct Agent {
    pub(crate) inner: Arc<Mutex<Inner>>,
    pub(crate) bus: Arc<parking_lot::Mutex<HashMap<String, broadcast::Sender<SseEvent>>>>,
    pub(crate) grok_bin: PathBuf,
    pub(crate) grok_home: PathBuf,
    pub(crate) permission_mode: String,
    pub(crate) pid_file: PathBuf,
    pub(crate) leader_json: PathBuf,
    pub(crate) ask_bridge: Option<AskBridge>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SseEvent {
    pub kind: String,
    pub data: String,
}

pub struct RuntimeView {
    pub agent_ok: bool,
    pub models: Vec<ModelInfo>,
    pub current_model: String,
    pub commands: Vec<SlashCommand>,
}

pub struct NewSession {
    pub id: String,
    pub cwd: String,
    pub model: String,
}

#[derive(Serialize)]
pub struct PromptOutcome {
    pub queued: bool,
    pub queue: Vec<QueueItem>,
}

pub(crate) struct Inner {
    pub(crate) child: Option<Child>,
    pub(crate) stdin: Option<ChildStdin>,
    pub(crate) next_id: u64,
    pub(crate) pending: HashMap<u64, oneshot::Sender<Result<Value, String>>>,
    pub(crate) in_flight: HashMap<u64, String>,
    pub(crate) initialized: bool,
    pub(crate) image_ok: bool,
    pub(crate) models: Vec<ModelInfo>,
    pub(crate) current_model: String,
    pub(crate) commands: Vec<SlashCommand>,
    pub(crate) sessions: HashMap<String, Live>,
    pub(crate) child_pid: Option<u32>,
    pub(crate) web_active_id: Option<String>,
    pub(crate) question_tx: HashMap<String, oneshot::Sender<QuestionReply>>,
    pub(crate) question_rx: HashMap<String, oneshot::Receiver<QuestionReply>>,
}

pub(crate) struct Live {
    pub(crate) cwd: String,
    pub(crate) loaded: bool,
    pub(crate) running: bool,
    pub(crate) user_emitted: bool,
    pub(crate) queue: VecDeque<QueueItem>,
    pub(crate) resume: Option<QueueItem>,
    pub(crate) perms: HashMap<String, PendingPerm>,
    pub(crate) questions: HashMap<String, PendingQuestion>,
    pub(crate) usage: TokenUsage,
    pub(crate) model: String,
    pub(crate) parser: Parser,
    pub(crate) last_tool_id: String,
    pub(crate) effort: String,
}

pub(crate) struct PendingPerm {
    pub(crate) rpc_id: Value,
    pub(crate) options: Vec<PermOpt>,
}

#[derive(Clone)]
pub(crate) struct PermOpt {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) name: String,
}

impl Agent {
    #[must_use]
    pub fn new(
        grok_bin: PathBuf,
        grok_home: PathBuf,
        permission_mode: String,
        agent_pid_file: PathBuf,
        leader_json: PathBuf,
    ) -> Self {
        occupy::reap_idle_leftover(&agent_pid_file, &grok_home);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                child: None,
                stdin: None,
                next_id: 1,
                pending: HashMap::new(),
                in_flight: HashMap::new(),
                initialized: false,
                image_ok: false,
                models: Vec::new(),
                current_model: String::new(),
                commands: Vec::new(),
                sessions: HashMap::new(),
                child_pid: None,
                web_active_id: occupy::read_web_active(&occupy::web_active_path(&leader_json)),
                question_tx: HashMap::new(),
                question_rx: HashMap::new(),
            })),
            bus: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            grok_bin,
            grok_home,
            permission_mode,
            pid_file: agent_pid_file,
            leader_json,
            ask_bridge: None,
        }
    }

    #[must_use]
    pub fn with_ask_bridge(mut self, bridge: AskBridge) -> Self {
        self.ask_bridge = Some(bridge);
        self
    }

    #[must_use]
    pub fn subscribe(&self, session_id: &str) -> broadcast::Receiver<SseEvent> {
        let mut bus = self.bus.lock();
        if let Some(tx) = bus.get(session_id) {
            return tx.subscribe();
        }
        let (tx, rx) = broadcast::channel(256);
        bus.insert(session_id.to_string(), tx);
        rx
    }

    pub async fn runtime(&self) -> RuntimeView {
        let docs = ggok_core::slash_docs::from_docs(&self.grok_home);
        let cached = ggok_core::parse::models_from_cache(&self.grok_home);
        let g = self.inner.lock().await;
        let agent_ok = g.initialized && process::child_alive(g.child_pid);
        let models = if g.models.is_empty() {
            cached
        } else {
            g.models.clone()
        };
        RuntimeView {
            agent_ok,
            models,
            current_model: g.current_model.clone(),
            commands: slash::merge_commands(docs, g.commands.clone()),
        }
    }

    pub async fn child_pid(&self) -> Option<u32> {
        self.inner.lock().await.child_pid
    }

    pub async fn live_map(&self) -> HashMap<String, LiveView> {
        let g = self.inner.lock().await;
        g.sessions
            .iter()
            .map(|(id, s)| {
                (
                    id.clone(),
                    LiveView {
                        loaded: s.loaded,
                        running: s.running,
                        model: s.model.clone(),
                        effort: s.effort.clone(),
                    },
                )
            })
            .collect()
    }

    pub async fn live_view(&self, id: &str) -> Option<LiveView> {
        let g = self.inner.lock().await;
        g.sessions.get(id).map(|s| LiveView {
            loaded: s.loaded,
            running: s.running,
            model: s.model.clone(),
            effort: s.effort.clone(),
        })
    }

    pub async fn live_blocks(&self, id: &str) -> Vec<Block> {
        let g = self.inner.lock().await;
        g.sessions
            .get(id)
            .filter(|s| s.running)
            .map(|s| s.parser.snapshot_blocks())
            .unwrap_or_default()
    }

    pub async fn live_work_started_ms(&self, id: &str) -> Option<u64> {
        let g = self.inner.lock().await;
        g.sessions.get(id).and_then(|s| s.parser.work_started_ms())
    }

    pub async fn live_usage(&self, id: &str) -> Option<(TokenUsage, u64)> {
        let g = self.inner.lock().await;
        g.sessions.get(id).map(|s| {
            let mut usage = s.parser.usage_snapshot();
            if !usage.recorded && s.usage.recorded {
                usage = s.usage.clone();
            }
            (usage, s.parser.context_tokens())
        })
    }

    pub(crate) fn emit_live(&self, session_id: &str, running: bool) {
        self.emit(
            session_id,
            "live",
            &json!({
                "source": "attached",
                "writable": true,
                "running": running,
            }),
        );
    }

    pub(crate) fn emit<T: Serialize>(&self, session_id: &str, kind: &str, value: &T) {
        let Ok(data) = serde_json::to_string(value) else {
            return;
        };
        let bus = self.bus.lock();
        if let Some(tx) = bus.get(session_id) {
            let _ = tx.send(SseEvent {
                kind: kind.to_string(),
                data,
            });
        }
    }
}

pub(crate) fn live_entry<'a>(inner: &'a mut Inner, id: &str, cwd: &str) -> &'a mut Live {
    inner
        .sessions
        .entry(id.to_string())
        .or_insert_with(|| Live {
            cwd: cwd.to_string(),
            loaded: false,
            running: false,
            user_emitted: false,
            queue: VecDeque::new(),
            resume: None,
            perms: HashMap::new(),
            questions: HashMap::new(),
            usage: TokenUsage::default(),
            model: String::new(),
            parser: Parser::new(),
            last_tool_id: String::new(),
            effort: String::new(),
        })
}

pub(crate) fn reset_parser_keep_usage(sess: &mut Live) {
    let usage = sess.parser.usage_snapshot();
    let ctx = sess.parser.context_tokens();
    sess.parser = Parser::new();
    if usage.recorded {
        sess.parser.replace_usage(usage.clone());
        sess.usage = usage;
    }
    sess.parser.set_context_tokens(ctx);
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}
