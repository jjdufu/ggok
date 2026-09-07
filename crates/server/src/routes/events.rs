use crate::http::valid_id;
use crate::service::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use ggok_agent::tail;
use ggok_agent::{Agent, SseEvent};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::MissedTickBehavior;
use tokio_stream::wrappers::ReceiverStream;

type EventTx = tokio::sync::mpsc::Sender<Result<Event, Infallible>>;

const SSE_OUT_CAP: usize = 256;
const PING_SECS: u64 = 10;

pub(crate) async fn api_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    let (tx, out_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(SSE_OUT_CAP);
    let live = json!({
        "source": occ.source.as_str(),
        "writable": occ.writable,
        "running": occ.running,
    });
    if let Ok(data) = serde_json::to_string(&live) {
        let _ = tx.send(Ok(Event::default().event("live").data(data))).await;
    }
    if occ.source.is_spectator() {
        stream_cli_events(state, id, tx).await;
    } else {
        stream_agent_events(state, id, tx).await;
    }
    Sse::new(ReceiverStream::new(out_rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

async fn stream_agent_events(state: Arc<AppState>, id: String, tx: EventTx) {
    let mut rx = state.agent.subscribe(&id);
    let queue = state.agent.queue_list(&id).await;
    if let Ok(data) = serde_json::to_string(&queue) {
        let _ = tx
            .send(Ok(Event::default().event("queue").data(data)))
            .await;
    }
    let questions = state.agent.pending_questions(&id).await;
    if let Ok(data) = serde_json::to_string(&questions) {
        let _ = tx
            .send(Ok(Event::default().event("questions").data(data)))
            .await;
    }
    seed_session_usage(&state, &id, &tx).await;
    for block in state.agent.live_blocks(&id).await {
        if let Ok(data) = serde_json::to_string(&block) {
            let _ = tx
                .send(Ok(Event::default().event("block").data(data)))
                .await;
        }
    }
    let agent = state.agent.clone();
    tokio::spawn(async move {
        let mut ping = tokio::time::interval(Duration::from_secs(PING_SECS));
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                ev = rx.recv() => {
                    match ev {
                        Ok(ev) => {
                            if !try_push(&tx, &ev.kind, ev.data) {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            if !push_resync(&agent, &id, &tx).await {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = ping.tick() => {
                    if !try_push(&tx, "ping", "{}") {
                        break;
                    }
                }
            }
        }
    });
}

fn try_push(tx: &EventTx, kind: &str, data: impl Into<String>) -> bool {
    let event = Event::default().event(kind).data(data.into());
    match tx.try_send(Ok(event)) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) => {
            let _ = tx.try_send(Ok(Event::default().event("resync").data("{}")));
            true
        }
        Err(TrySendError::Closed(_)) => false,
    }
}

async fn push_resync(agent: &Agent, id: &str, tx: &EventTx) -> bool {
    if !try_push(tx, "resync", "{}") {
        return false;
    }
    for block in agent.live_blocks(id).await {
        let Ok(data) = serde_json::to_string(&block) else {
            continue;
        };
        if !try_push(tx, "block", data) {
            return false;
        }
    }
    true
}

async fn stream_cli_events(state: Arc<AppState>, id: String, tx: EventTx) {
    let Some(meta) = state.session(&id) else {
        return;
    };
    let path = meta.dir.join("updates.jsonl");
    let start_offset = std::fs::metadata(&path).map_or(0, |m| m.len());
    let our = ggok_core::occupy::our_runtime_pid(state.agent.child_pid().await);
    let leftover_pid_file = state.agent_pid_file.clone();
    let session_dir = meta.dir.clone();
    let (sse_tx, mut sse_rx) = tokio::sync::mpsc::channel::<SseEvent>(SSE_OUT_CAP);
    tokio::spawn(async move {
        tail::run(
            tail::TailJob {
                path,
                grok_home: state.grok_home.clone(),
                session_id: id,
                session_dir,
                leftover_pid_file,
                our_runtime_pid: our,
                start_offset,
                model: meta.model.clone(),
            },
            sse_tx,
        )
        .await;
    });
    tokio::spawn(async move {
        let mut ping = tokio::time::interval(Duration::from_secs(PING_SECS));
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                ev = sse_rx.recv() => {
                    let Some(ev) = ev else { break; };
                    if !try_push(&tx, &ev.kind, ev.data) {
                        break;
                    }
                }
                _ = ping.tick() => {
                    if !try_push(&tx, "ping", "{}") {
                        break;
                    }
                }
            }
        }
    });
}

async fn seed_session_usage(
    state: &AppState,
    id: &str,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
) {
    if let Some(meta) = state.session(id)
        && let Ok(parsed) = state.parsed(&meta)
    {
        if parsed.usage.recorded
            && let Ok(data) = serde_json::to_string(&parsed.usage)
        {
            let _ = tx
                .send(Ok(Event::default().event("usage").data(data)))
                .await;
        }
        if parsed.context_tokens > 0 {
            let window = ggok_core::parse::context_window(&state.grok_home, &meta.model);
            let ctx = json!({ "used": parsed.context_tokens, "window": window });
            if let Ok(data) = serde_json::to_string(&ctx) {
                let _ = tx
                    .send(Ok(Event::default().event("context").data(data)))
                    .await;
            }
        }
    }
    if let Some((usage, ctx)) = state.agent.live_usage(id).await {
        if usage.recorded
            && let Ok(data) = serde_json::to_string(&usage)
        {
            let _ = tx
                .send(Ok(Event::default().event("usage").data(data)))
                .await;
        }
        if ctx > 0 {
            let model = state
                .agent
                .live_view(id)
                .await
                .map(|v| v.model)
                .unwrap_or_default();
            let window = ggok_core::parse::context_window(&state.grok_home, &model);
            let body = json!({ "used": ctx, "window": window });
            if let Ok(data) = serde_json::to_string(&body) {
                let _ = tx
                    .send(Ok(Event::default().event("context").data(data)))
                    .await;
            }
        }
    }
}
