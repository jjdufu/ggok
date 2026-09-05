use crate::SseEvent;
use ggok_core::occupy::{self, Source};
use ggok_core::parse::{self, Ingest, Parser};
use ggok_core::types::Block;
use serde_json::{Value, json};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

const TAIL_MS: u64 = 150;

pub struct TailJob {
    pub path: PathBuf,
    pub grok_home: PathBuf,
    pub session_id: String,
    pub agent_pid: Option<u32>,
    pub start_offset: u64,
    pub model: String,
}

pub async fn run(job: TailJob, tx: Sender<SseEvent>) {
    let mut offset = job.start_offset;
    let mut parser = Parser::new();
    let mut incomplete = String::new();
    warm_parser(&job.path, offset, &mut parser);
    let mut last_source = Source::Cli;
    let mut last_running = true;
    loop {
        if tx.is_closed() {
            break;
        }
        let cli = occupy::cli_sessions(&job.grok_home, job.agent_pid);
        let occ = occupy::classify(&job.session_id, None, &cli);
        if occ.source != last_source || occ.running != last_running {
            last_source = occ.source;
            last_running = occ.running;
            let live = json!({
                "source": occ.source.as_str(),
                "writable": occ.writable,
                "running": occ.running,
            });
            if send_event(&tx, "live", &live).await.is_err() {
                break;
            }
        }
        if let Ok(lines) = read_new(&job.path, &mut offset, &mut incomplete) {
            for line in lines {
                if let Ok(obj) = serde_json::from_str::<Value>(&line) {
                    emit_line(&mut parser, &obj, &tx, &job.grok_home, &job.model).await;
                }
            }
        }
        if occ.source != Source::Cli && !occ.running {
            tokio::time::sleep(Duration::from_millis(TAIL_MS)).await;
            if occupy::cli_sessions(&job.grok_home, job.agent_pid).contains_key(&job.session_id) {
                continue;
            }
            break;
        }
        tokio::time::sleep(Duration::from_millis(TAIL_MS)).await;
    }
}

fn warm_parser(path: &Path, stop_at: u64, parser: &mut Parser) {
    let Ok(mut file) = std::fs::File::open(path) else {
        return;
    };
    let mut buf = String::new();
    let _ = file.read_to_string(&mut buf);
    let mut seen = 0_u64;
    for line in buf.lines() {
        let span = u64::try_from(line.len().saturating_add(1)).unwrap_or(0);
        if seen + span > stop_at && stop_at > 0 {
            break;
        }
        seen = seen.saturating_add(span);
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<Value>(line) {
            parse::apply_event(parser, &obj);
        }
    }
}

fn read_new(
    path: &Path,
    offset: &mut u64,
    incomplete: &mut String,
) -> std::io::Result<Vec<String>> {
    let meta = std::fs::metadata(path)?;
    let size = meta.len();
    if size < *offset {
        *offset = 0;
        incomplete.clear();
    }
    if size == *offset {
        return Ok(Vec::new());
    }
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(*offset))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    *offset = size;
    incomplete.push_str(&buf);
    let mut lines = Vec::new();
    while let Some(pos) = incomplete.find('\n') {
        let line: String = incomplete.drain(..=pos).collect();
        let line = line.trim_end_matches(['\n', '\r']).to_string();
        if !line.is_empty() {
            lines.push(line);
        }
    }
    Ok(lines)
}

async fn emit_line(
    parser: &mut Parser,
    obj: &Value,
    tx: &Sender<SseEvent>,
    grok_home: &Path,
    model: &str,
) {
    let before = parser.last_block();
    let before_ctx = parser.context_tokens();
    parse::apply_event(parser, obj);
    if parser.context_tokens() != before_ctx {
        let window = parse::context_window(grok_home, model);
        let _ = send_event(
            tx,
            "context",
            &json!({ "used": parser.context_tokens(), "window": window }),
        )
        .await;
    }
    let ingest = classify_update(obj);
    match ingest {
        Ingest::Text => {
            if let Some(block) = parser.open_text().or_else(|| parser.last_block()) {
                let _ = send_event(tx, "block", &block).await;
            }
        }
        Ingest::Tool => {
            if let Some(block) = tool_from_update(parser, obj) {
                let _ = send_event(tx, "block", &block).await;
            }
        }
        Ingest::TurnEnd => {
            if let Some(block) = parser.last_block() {
                let _ = send_event(tx, "block", &block).await;
            }
            let _ = send_event(tx, "usage", &parser.usage_snapshot()).await;
        }
        Ingest::Usage => {
            let _ = send_event(tx, "usage", &parser.usage_snapshot()).await;
        }
        Ingest::None => {
            let _ = before;
        }
    }
}

fn classify_update(obj: &Value) -> Ingest {
    let kind = obj
        .pointer("/params/update/sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("");
    match kind {
        "user_message_chunk" | "agent_thought_chunk" | "agent_message_chunk" => Ingest::Text,
        "tool_call" | "tool_call_update" => Ingest::Tool,
        "turn_completed" => Ingest::TurnEnd,
        "usage_update" => Ingest::Usage,
        _ => Ingest::None,
    }
}

fn tool_from_update(parser: &Parser, obj: &Value) -> Option<Block> {
    let id = obj
        .pointer("/params/update/toolCallId")
        .and_then(Value::as_str)
        .unwrap_or("");
    if id.is_empty() {
        return parser.last_block();
    }
    parser.tool(id)
}

async fn send_event<T: serde::Serialize>(
    tx: &Sender<SseEvent>,
    kind: &str,
    value: &T,
) -> Result<(), ()> {
    let Ok(data) = serde_json::to_string(value) else {
        return Ok(());
    };
    tx.send(SseEvent {
        kind: kind.to_string(),
        data,
    })
    .await
    .map_err(|_| ())
}
