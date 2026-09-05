use crate::http::valid_id;
use crate::service::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use ggok_agent::SseEvent;
use ggok_agent::tail;
use ggok_core::occupy::Source;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

type EventTx = tokio::sync::mpsc::Sender<Result<Event, Infallible>>;

pub(crate) async fn api_events(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    if !valid_id(&id) {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    }
    let occ = super::occupancy(&state, &id).await;
    let (tx, out_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let live = json!({
        "source": occ.source.as_str(),
        "writable": occ.writable,
        "running": occ.running,
    });
    if let Ok(data) = serde_json::to_string(&live) {
        let _ = tx.send(Ok(Event::default().event("live").data(data))).await;
    }
    match occ.source {
        Source::Agent | Source::Disk => stream_agent_events(state, id, tx).await,
        Source::Cli => stream_cli_events(state, id, tx).await,
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
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let event = Event::default().event(ev.kind).data(ev.data);
                    if tx.send(Ok(event)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    if tx
                        .send(Ok(Event::default().event("resync").data("{}")))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    let mut dead = false;
                    for block in agent.live_blocks(&id).await {
                        if let Ok(data) = serde_json::to_string(&block)
                            && tx
                                .send(Ok(Event::default().event("block").data(data)))
                                .await
                                .is_err()
                        {
                            dead = true;
                            break;
                        }
                    }
                    if dead {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn stream_cli_events(state: Arc<AppState>, id: String, tx: EventTx) {
    let Some(meta) = state.session(&id) else {
        return;
    };
    let path = meta.dir.join("updates.jsonl");
    let start_offset = std::fs::metadata(&path).map_or(0, |m| m.len());
    let agent_pid = ggok_core::occupy::agent_pid(&state.agent_pid_file, state.agent.child_pid().await);
    let (sse_tx, mut sse_rx) = tokio::sync::mpsc::channel::<SseEvent>(64);
    tokio::spawn(async move {
        tail::run(
            tail::TailJob {
                path,
                grok_home: state.grok_home.clone(),
                session_id: id,
                agent_pid,
                start_offset,
                model: meta.model.clone(),
            },
            sse_tx,
        )
        .await;
    });
    tokio::spawn(async move {
        while let Some(ev) = sse_rx.recv().await {
            let event = Event::default().event(ev.kind).data(ev.data);
            if tx.send(Ok(event)).await.is_err() {
                break;
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
        if usage.recorded && let Ok(data) = serde_json::to_string(&usage) {
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
