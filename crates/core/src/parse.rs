use crate::types::{Block, EffortInfo, ModelInfo, ModelUsageRow, TokenUsage, ToolDetail};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const PREVIEW_CHARS: usize = 200;

#[derive(Debug, Clone)]
pub struct ParsedSession {
    pub blocks: Vec<Block>,
    pub usage: TokenUsage,
    pub context_tokens: u64,
    pub work_started_ms: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextKind {
    User,
    Thought,
    Assistant,
}

struct TextBuf {
    kind: TextKind,
    prompt_id: String,
    text: String,
}

pub struct Parser {
    blocks: Vec<Block>,
    buf: Option<TextBuf>,
    tools: HashMap<String, usize>,
    usage: TokenUsage,
    models: BTreeMap<String, ModelUsageRow>,
    context_tokens: u64,
    turn_start_ms: Option<u64>,
    last_ts_ms: Option<u64>,
    current_prompt_id: String,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            buf: None,
            tools: HashMap::new(),
            usage: TokenUsage::default(),
            models: BTreeMap::new(),
            context_tokens: 0,
            turn_start_ms: None,
            last_ts_ms: None,
            current_prompt_id: String::new(),
        }
    }

    pub fn note_time(&mut self, ts_ms: Option<u64>) {
        let Some(ts) = ts_ms else {
            return;
        };
        self.last_ts_ms = Some(ts);
        if self.turn_start_ms.is_none() {
            self.turn_start_ms = Some(ts);
        }
    }

    #[must_use]
    pub fn last_block(&self) -> Option<Block> {
        self.blocks.last().cloned()
    }

    #[must_use]
    pub fn work_started_ms(&self) -> Option<u64> {
        self.turn_start_ms
    }

    fn flush_text(&mut self) {
        if let Some(buf) = self.buf.take() {
            let block = match buf.kind {
                TextKind::User => Block::User {
                    prompt_id: buf.prompt_id,
                    text: buf.text,
                    files: Vec::new(),
                },
                TextKind::Thought => Block::Thought {
                    prompt_id: buf.prompt_id,
                    text: buf.text,
                },
                TextKind::Assistant => Block::Assistant {
                    prompt_id: buf.prompt_id,
                    text: buf.text,
                },
            };
            self.blocks.push(block);
        }
    }

    fn push_text(&mut self, kind: TextKind, prompt_id: String, text: &str) {
        let continue_buf = self.buf.as_ref().is_some_and(|buf| {
            buf.kind == kind
                && (buf.prompt_id == prompt_id || prompt_id.is_empty() || buf.prompt_id.is_empty())
        });
        if continue_buf {
            if let Some(buf) = self.buf.as_mut() {
                if buf.prompt_id.is_empty() && !prompt_id.is_empty() {
                    buf.prompt_id = prompt_id;
                }
                buf.text.push_str(text);
            }
            return;
        }
        // Session load / MCP reinit can replay the same user_message_chunk
        // into a live parser that already has this turn.
        if kind == TextKind::User && !text.is_empty() && self.open_turn_has_user(text) {
            if !prompt_id.is_empty() {
                self.backfill_user_prompt(&prompt_id);
                self.current_prompt_id.clone_from(&prompt_id);
            }
            return;
        }
        self.flush_text();
        if !prompt_id.is_empty() {
            self.current_prompt_id.clone_from(&prompt_id);
        }
        self.buf = Some(TextBuf {
            kind,
            prompt_id,
            text: text.to_string(),
        });
    }

    fn open_turn_has_user(&self, text: &str) -> bool {
        if self
            .buf
            .as_ref()
            .is_some_and(|buf| buf.kind == TextKind::User && buf.text == text)
        {
            return true;
        }
        let start = self
            .blocks
            .iter()
            .rposition(|b| matches!(b, Block::TurnEnd { .. }))
            .map_or(0, |i| i + 1);
        self.blocks[start..]
            .iter()
            .any(|b| matches!(b, Block::User { text: t, .. } if t == text))
    }

    fn backfill_user_prompt(&mut self, prompt_id: &str) {
        if prompt_id.is_empty() {
            return;
        }
        if let Some(buf) = self.buf.as_mut()
            && buf.kind == TextKind::User
            && buf.prompt_id.is_empty()
        {
            buf.prompt_id = prompt_id.to_string();
            return;
        }
        if let Some(Block::User { prompt_id: pid, .. }) = self.blocks.last_mut()
            && pid.is_empty()
        {
            *pid = prompt_id.to_string();
        }
    }

    fn start_tool(&mut self, id: String, title: String, raw_input: &Value) {
        self.flush_text();
        if self.tools.contains_key(&id) {
            return;
        }
        let preview = input_preview(raw_input);
        let idx = self.blocks.len();
        self.tools.insert(id.clone(), idx);
        self.blocks.push(Block::Tool {
            id,
            title,
            status: String::new(),
            input_preview: preview,
            prompt_id: self.current_prompt_id.clone(),
            result_count: None,
        });
    }

    fn update_tool(
        &mut self,
        id: &str,
        title: Option<String>,
        status: Option<String>,
        raw_input: Option<&Value>,
        content: Option<&Value>,
    ) {
        let Some(&idx) = self.tools.get(id) else {
            let title = title.unwrap_or_default();
            let raw = raw_input.cloned().unwrap_or(Value::Null);
            self.start_tool(id.to_string(), title, &raw);
            self.update_tool(id, None, status, None, content);
            return;
        };
        let Some(Block::Tool {
            title: t,
            status: s,
            input_preview,
            result_count,
            prompt_id,
            ..
        }) = self.blocks.get_mut(idx)
        else {
            return;
        };
        if prompt_id.is_empty() && !self.current_prompt_id.is_empty() {
            prompt_id.clone_from(&self.current_prompt_id);
        }
        if let Some(title) = title
            && !title.is_empty()
        {
            *t = title;
        }
        if let Some(status) = status
            && !status.is_empty()
        {
            *s = status;
        }
        if let Some(raw) = raw_input
            && input_preview.is_empty()
        {
            *input_preview = crate::parse::input_preview(raw);
        }
        if let Some(count) = content.and_then(count_results) {
            *result_count = Some(count);
        }
    }

    fn turn_end(&mut self, prompt_id: String, cancelled: bool, elapsed_ms: Option<u64>) {
        self.flush_text();
        if !prompt_id.is_empty() {
            self.backfill_user_prompt(&prompt_id);
        }
        let from_ts = match (self.turn_start_ms, self.last_ts_ms) {
            (Some(start), Some(end)) if end >= start => end.saturating_sub(start),
            _ => 0,
        };
        let duration_ms = elapsed_ms.unwrap_or(0).max(from_ts);
        self.turn_start_ms = None;
        self.last_ts_ms = None;
        self.blocks.push(Block::TurnEnd {
            prompt_id,
            duration_ms,
            cancelled,
        });
    }

    fn add_usage(&mut self, usage: &Value) {
        self.usage.recorded = true;
        add_usage_into(&mut self.usage, usage);
        if let Some(map) = usage
            .get("modelUsage")
            .or_else(|| usage.get("model_usage"))
            .and_then(Value::as_object)
        {
            for (name, row) in map {
                let entry = self.models.entry(name.clone()).or_default();
                if entry.model.is_empty() {
                    entry.model.clone_from(name);
                }
                add_model_usage(entry, row);
            }
        }
    }

    fn finish(mut self) -> ParsedSession {
        self.flush_text();
        compact_duplicate_open_users(&mut self.blocks);
        let mut usage = self.usage;
        usage.models = self.models.into_values().collect();
        ParsedSession {
            blocks: self.blocks,
            usage,
            context_tokens: self.context_tokens,
            work_started_ms: self.turn_start_ms,
        }
    }

    pub fn ingest_at(&mut self, update: &Value, prompt_id: &str, ts_ms: Option<u64>) -> Ingest {
        ingest_update(self, update, prompt_id, ts_ms)
    }

    #[must_use]
    pub fn snapshot_blocks(&self) -> Vec<Block> {
        let mut out = self.blocks.clone();
        if let Some(open) = self.open_text() {
            out.push(open);
        }
        compact_duplicate_open_users(&mut out);
        out
    }

    #[must_use]
    pub fn open_text(&self) -> Option<Block> {
        self.buf.as_ref().map(|buf| match buf.kind {
            TextKind::User => Block::User {
                prompt_id: buf.prompt_id.clone(),
                text: buf.text.clone(),
                files: Vec::new(),
            },
            TextKind::Thought => Block::Thought {
                prompt_id: buf.prompt_id.clone(),
                text: buf.text.clone(),
            },
            TextKind::Assistant => Block::Assistant {
                prompt_id: buf.prompt_id.clone(),
                text: buf.text.clone(),
            },
        })
    }

    #[must_use]
    pub fn tool(&self, id: &str) -> Option<Block> {
        let idx = *self.tools.get(id)?;
        self.blocks.get(idx).cloned()
    }

    #[must_use]
    pub fn usage_snapshot(&self) -> TokenUsage {
        let mut usage = self.usage.clone();
        usage.models = self.models.values().cloned().collect();
        usage
    }

    pub fn replace_usage(&mut self, mut snap: TokenUsage) {
        self.models.clear();
        for row in snap.models.drain(..) {
            if !row.model.is_empty() {
                self.models.insert(row.model.clone(), row);
            }
        }
        snap.models.clear();
        self.usage = snap;
    }

    #[must_use]
    pub fn context_tokens(&self) -> u64 {
        self.context_tokens
    }

    pub fn set_context_tokens(&mut self, n: u64) {
        if n > 0 {
            self.context_tokens = n;
        }
    }

    pub fn note_meta(&mut self, meta: &Value) {
        if let Some(n) = json_u64_opt(meta, &["totalTokens", "total_tokens"]) {
            self.context_tokens = n;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ingest {
    None,
    Text,
    Tool,
    TurnEnd,
    Usage,
}

fn ingest_update(
    parser: &mut Parser,
    update: &Value,
    prompt_id: &str,
    ts_ms: Option<u64>,
) -> Ingest {
    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut prompt_id = prompt_id.to_string();
    if prompt_id.is_empty() {
        prompt_id = update
            .get("prompt_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    if !prompt_id.is_empty() {
        parser.current_prompt_id.clone_from(&prompt_id);
    }
    match kind {
        "user_message_chunk" => {
            ingest_text(parser, TextKind::User, prompt_id, update, ts_ms, false)
        }
        "agent_thought_chunk" => {
            ingest_text(parser, TextKind::Thought, prompt_id, update, ts_ms, true)
        }
        "agent_message_chunk" => {
            ingest_text(parser, TextKind::Assistant, prompt_id, update, ts_ms, true)
        }
        "tool_call" => ingest_tool_call(parser, &prompt_id, update, ts_ms),
        "tool_call_update" => ingest_tool_update(parser, update, ts_ms),
        "turn_completed" => ingest_turn_completed(parser, prompt_id, update, ts_ms),
        _ => ingest_usage_field(parser, update),
    }
}

fn ingest_text(
    parser: &mut Parser,
    kind: TextKind,
    prompt_id: String,
    update: &Value,
    ts_ms: Option<u64>,
    backfill: bool,
) -> Ingest {
    parser.note_time(ts_ms);
    if backfill {
        parser.backfill_user_prompt(&prompt_id);
    }
    let text = content_text(update.get("content"));
    parser.push_text(kind, prompt_id, &text);
    Ingest::Text
}

fn ingest_tool_call(
    parser: &mut Parser,
    prompt_id: &str,
    update: &Value,
    ts_ms: Option<u64>,
) -> Ingest {
    parser.note_time(ts_ms);
    parser.backfill_user_prompt(prompt_id);
    let id = update
        .get("toolCallId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if id.is_empty() {
        return Ingest::None;
    }
    let title = update
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let raw = update.get("rawInput").cloned().unwrap_or(Value::Null);
    parser.start_tool(id, title, &raw);
    Ingest::Tool
}

fn ingest_tool_update(parser: &mut Parser, update: &Value, ts_ms: Option<u64>) -> Ingest {
    parser.note_time(ts_ms);
    let id = update
        .get("toolCallId")
        .and_then(Value::as_str)
        .unwrap_or("");
    if id.is_empty() {
        return Ingest::None;
    }
    let title = update
        .get("title")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let status = update
        .get("status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    parser.update_tool(
        id,
        title,
        status,
        update.get("rawInput"),
        update.get("content"),
    );
    Ingest::Tool
}

fn ingest_turn_completed(
    parser: &mut Parser,
    prompt_id: String,
    update: &Value,
    ts_ms: Option<u64>,
) -> Ingest {
    parser.note_time(ts_ms);
    if let Some(usage) = update.get("usage") {
        parser.add_usage(usage);
    }
    let stop = update
        .get("stop_reason")
        .or_else(|| update.get("stopReason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let cancelled = matches!(
        stop,
        "cancelled" | "canceled" | "interrupted" | "user_cancelled"
    );
    let elapsed = json_u64_opt(update, &["elapsed_ms", "elapsedMs"]);
    parser.turn_end(prompt_id, cancelled, elapsed);
    Ingest::TurnEnd
}

fn ingest_usage_field(parser: &mut Parser, update: &Value) -> Ingest {
    let Some(usage) = update.get("usage") else {
        return Ingest::None;
    };
    parser.add_usage(usage);
    Ingest::Usage
}

fn json_u64(v: &Value, keys: &[&str]) -> u64 {
    json_u64_opt(v, keys).unwrap_or(0)
}

fn json_u64_opt(v: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(n) = v.get(*key).and_then(Value::as_u64) {
            return Some(n);
        }
        if let Some(n) = v.get(*key).and_then(Value::as_i64) {
            return u64::try_from(n).ok();
        }
    }
    None
}

#[must_use]
pub fn fallback_window(model: &str) -> u64 {
    let id = model.strip_suffix("-build").unwrap_or(model);
    if id.contains("4.6") || id.contains("4.5") || id.contains("grok-4") {
        500_000
    } else {
        200_000
    }
}

#[must_use]
pub fn models_from_cache(grok_home: &Path) -> Vec<ModelInfo> {
    let path = grok_home.join("models_cache.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return Vec::new();
    };
    let Some(map) = v.get("models").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, entry) in map {
        let info = entry.get("info").unwrap_or(entry);
        if info.get("hidden").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let name = info
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_string();
        let description = info
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let effort = info
            .get("reasoning_effort")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let efforts = info
            .get("reasoning_efforts")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| {
                        Some(EffortInfo {
                            id: x.get("id")?.as_str()?.to_string(),
                            label: x
                                .get("label")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            description: x
                                .get("description")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let context_window = info
            .get("context_window")
            .and_then(Value::as_u64)
            .filter(|n| *n > 0);
        out.push(ModelInfo {
            id: id.clone(),
            name,
            description,
            effort,
            efforts,
            context_window,
        });
    }
    out
}

#[must_use]
pub fn context_window(grok_home: &Path, model: &str) -> u64 {
    let key = model.strip_suffix("-build").unwrap_or(model);
    let path = grok_home.join("models_cache.json");
    if let Ok(raw) = fs::read_to_string(&path)
        && let Ok(v) = serde_json::from_str::<Value>(&raw)
    {
        let pointer = format!("/models/{key}/info/context_window");
        if let Some(n) = v
            .pointer(&pointer)
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
        {
            return n;
        }
    }
    fallback_window(model)
}

fn add_usage_into(dst: &mut TokenUsage, usage: &Value) {
    dst.input_tokens += json_u64(usage, &["inputTokens", "input_tokens"]);
    dst.cached_tokens += json_u64(
        usage,
        &[
            "cachedReadTokens",
            "cached_read_tokens",
            "cache_read_input_tokens",
        ],
    );
    dst.cache_creation_tokens += json_u64(
        usage,
        &[
            "cacheCreationTokens",
            "cache_creation_tokens",
            "cache_creation_input_tokens",
        ],
    );
    dst.output_tokens += json_u64(usage, &["outputTokens", "output_tokens"]);
    dst.reasoning_tokens += json_u64(usage, &["reasoningTokens", "reasoning_tokens"]);
    dst.total_tokens += json_u64(usage, &["totalTokens", "total_tokens"]);
    dst.model_calls += json_u64(usage, &["modelCalls", "model_calls"]);
    dst.api_duration_ms += json_u64(usage, &["apiDurationMs", "api_duration_ms"]);
    dst.cost_usd_ticks += json_u64(
        usage,
        &["costUsdTicks", "cost_usd_ticks", "total_cost_usd_ticks"],
    );
    dst.num_turns += json_u64(usage, &["numTurns", "num_turns"]);
}

fn add_model_usage(dst: &mut ModelUsageRow, usage: &Value) {
    dst.input_tokens += json_u64(usage, &["inputTokens", "input_tokens"]);
    dst.cached_tokens += json_u64(
        usage,
        &[
            "cachedReadTokens",
            "cached_read_tokens",
            "cache_read_input_tokens",
        ],
    );
    dst.cache_creation_tokens += json_u64(
        usage,
        &[
            "cacheCreationTokens",
            "cache_creation_tokens",
            "cache_creation_input_tokens",
        ],
    );
    dst.output_tokens += json_u64(usage, &["outputTokens", "output_tokens"]);
    dst.reasoning_tokens += json_u64(usage, &["reasoningTokens", "reasoning_tokens"]);
    dst.total_tokens += json_u64(usage, &["totalTokens", "total_tokens"]);
    dst.model_calls += json_u64(usage, &["modelCalls", "model_calls"]);
    dst.api_duration_ms += json_u64(usage, &["apiDurationMs", "api_duration_ms"]);
    dst.cost_usd_ticks += json_u64(
        usage,
        &["costUsdTicks", "cost_usd_ticks", "total_cost_usd_ticks"],
    );
}

/// # Errors
/// Returns an error if the updates file cannot be opened.
pub fn parse_updates_file(path: &Path) -> anyhow::Result<ParsedSession> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(parse_reader(reader))
}

fn parse_reader<R: BufRead>(reader: R) -> ParsedSession {
    let mut parser = Parser::new();
    let mut last_incomplete: Option<String> = None;
    for line_res in reader.lines() {
        let Ok(line) = line_res else {
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(obj) => {
                last_incomplete = None;
                apply_event(&mut parser, &obj);
            }
            Err(_) => last_incomplete = Some(line),
        }
    }
    if let Some(line) = last_incomplete
        && let Ok(obj) = serde_json::from_str::<Value>(&line)
    {
        apply_event(&mut parser, &obj);
    }
    parser.finish()
}

pub fn apply_event(parser: &mut Parser, obj: &Value) {
    let params = obj.get("params").unwrap_or(&Value::Null);
    let update = params.get("update").unwrap_or(&Value::Null);
    let meta = params.get("_meta").unwrap_or(&Value::Null);
    let mut prompt_id = meta
        .get("promptId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if prompt_id.is_empty() {
        prompt_id = update
            .get("prompt_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    let ts = timestamp_ms(obj.get("timestamp")).or_else(|| timestamp_ms(params.get("timestamp")));
    parser.note_meta(meta);
    ingest_update(parser, update, &prompt_id, ts);
}

pub fn timestamp_ms(v: Option<&Value>) -> Option<u64> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(normalize_ts(n));
    }
    if let Some(n) = v.as_i64() {
        return u64::try_from(n).ok().map(normalize_ts);
    }
    if let Some(s) = v.as_str() {
        return chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .and_then(|dt| u64::try_from(dt.timestamp_millis().max(0)).ok());
    }
    None
}

fn normalize_ts(n: u64) -> u64 {
    if n >= 1_000_000_000_000 {
        n
    } else {
        n.saturating_mul(1000)
    }
}

fn count_results(content: &Value) -> Option<u64> {
    for key in ["results", "items", "sources", "matches"] {
        if let Some(arr) = content.get(key).and_then(Value::as_array) {
            return u64::try_from(arr.len()).ok();
        }
    }
    None
}

fn content_text(content: Option<&Value>) -> String {
    let Some(content) = content else {
        return String::new();
    };
    if let Some(s) = content.as_str() {
        return s.to_string();
    }
    if let Some(s) = content.get("text").and_then(Value::as_str) {
        return s.to_string();
    }
    String::new()
}

fn input_preview(raw: &Value) -> String {
    if raw.is_null() {
        return String::new();
    }
    if let Some(s) = raw.as_str() {
        return truncate(s, PREVIEW_CHARS);
    }
    if let Some(obj) = raw.as_object() {
        for key in [
            "path",
            "target_file",
            "target_directory",
            "command",
            "query",
            "pattern",
            "url",
        ] {
            if let Some(v) = obj.get(key).and_then(Value::as_str) {
                return truncate(&format!("{key}: {v}"), PREVIEW_CHARS);
            }
        }
    }
    truncate(&raw.to_string(), PREVIEW_CHARS)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = s.chars().take(max).collect::<String>();
    out.push('…');
    out
}

const TOOL_LOG_CAP: u64 = 256 * 1024;

/// # Errors
/// Returns an error if the updates file cannot be opened.
pub fn extract_tool(path: &Path, tool_id: &str) -> anyhow::Result<Option<ToolDetail>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut content = Value::Null;
    let mut raw_output = Value::Null;
    let mut found = false;
    let mut last_incomplete: Option<String> = None;
    for line_res in reader.lines() {
        let Ok(line) = line_res else {
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(obj) => {
                last_incomplete = None;
                if apply_tool_extract(&obj, tool_id, &mut content, &mut raw_output) {
                    found = true;
                }
            }
            Err(_) => last_incomplete = Some(line),
        }
    }
    if let Some(line) = last_incomplete
        && let Ok(obj) = serde_json::from_str::<Value>(&line)
        && apply_tool_extract(&obj, tool_id, &mut content, &mut raw_output)
    {
        found = true;
    }
    let session_dir = path.parent().unwrap_or(path);
    normalize_tool_json(&mut content);
    normalize_tool_json(&mut raw_output);
    let log = tool_log_text(session_dir, tool_id, &raw_output);
    if found || !log.is_empty() {
        Ok(Some(ToolDetail {
            content,
            raw_output,
            log,
        }))
    } else {
        Ok(None)
    }
}

fn apply_tool_extract(
    obj: &Value,
    tool_id: &str,
    content: &mut Value,
    raw_output: &mut Value,
) -> bool {
    let update = obj
        .pointer("/params/update")
        .cloned()
        .unwrap_or(Value::Null);
    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("");
    if kind != "tool_call_update" && kind != "tool_call" {
        return false;
    }
    let id = update
        .get("toolCallId")
        .and_then(Value::as_str)
        .unwrap_or("");
    if id != tool_id {
        return false;
    }
    if let Some(c) = update.get("content") {
        *content = c.clone();
    }
    if let Some(r) = update.get("rawOutput") {
        *raw_output = r.clone();
    }
    true
}

fn tool_log_text(session_dir: &Path, tool_id: &str, raw: &Value) -> String {
    let named = session_dir.join("terminal").join(format!("{tool_id}.log"));
    let from_named = read_capped_log(&named);
    if !from_named.trim().is_empty() {
        return from_named;
    }
    for key in ["/output_file", "/Result/output_file"] {
        if let Some(p) = raw.pointer(key).and_then(Value::as_str) {
            let path = Path::new(p);
            if path.starts_with(session_dir) {
                let text = read_capped_log(path);
                if !text.trim().is_empty() {
                    return text;
                }
            }
        }
    }
    String::new()
}

fn read_capped_log(path: &Path) -> String {
    let Ok(meta) = fs::metadata(path) else {
        return String::new();
    };
    if !meta.is_file() {
        return String::new();
    }
    if meta.len() <= TOOL_LOG_CAP {
        return fs::read_to_string(path).unwrap_or_default();
    }
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let skip = meta.len() - TOOL_LOG_CAP;
    if file.seek(SeekFrom::Start(skip)).is_err() {
        return String::new();
    }
    let mut buf = String::new();
    let _ = file.read_to_string(&mut buf);
    if let Some(i) = buf.find('\n') {
        buf = buf[i + 1..].to_string();
    }
    format!("…\n{buf}")
}

fn normalize_tool_json(v: &mut Value) {
    match v {
        Value::Array(arr) => {
            if arr.len() >= 16 && arr.iter().all(|x| x.as_u64().is_some_and(|n| n <= 255)) {
                let bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|x| x.as_u64().and_then(|n| u8::try_from(n).ok()))
                    .collect();
                *v = Value::String(String::from_utf8_lossy(&bytes).into_owned());
                return;
            }
            for item in arr {
                normalize_tool_json(item);
            }
        }
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("image")
                && let Some(Value::String(s)) = map.get_mut("data")
            {
                let n = s.len();
                *s = format!("[image {n} bytes]");
            }
            if let Some(Value::Object(ic)) = map.get_mut("ImageContent")
                && let Some(Value::String(s)) = ic.get_mut("data")
            {
                let n = s.len();
                *s = format!("[image {n} bytes]");
            }
            for item in map.values_mut() {
                normalize_tool_json(item);
            }
        }
        _ => {}
    }
}

#[must_use]
pub fn merge_live_over_disk(disk: &[Block], live: &[Block]) -> Vec<Block> {
    if live.is_empty() {
        return disk.to_vec();
    }
    let last_turn_end_idx = disk
        .iter()
        .rposition(|b| matches!(b, Block::TurnEnd { .. }));
    let live_pid = live
        .iter()
        .map(Block::prompt_id)
        .find(|pid| !pid.is_empty());

    if let (Some(end_idx), Some(lpid)) = (last_turn_end_idx, live_pid)
        && end_idx == disk.len() - 1
        && let Some(Block::TurnEnd {
            prompt_id: dpid, ..
        }) = disk.get(end_idx)
        && !dpid.is_empty()
        && dpid == lpid
    {
        return disk.to_vec();
    }

    let cut = match last_turn_end_idx {
        Some(idx) => idx + 1,
        None => 0,
    };

    let mut out = disk[..cut].to_vec();
    out.extend_from_slice(live);
    compact_duplicate_open_users(&mut out);
    out
}

fn compact_duplicate_open_users(blocks: &mut Vec<Block>) {
    let mut out = Vec::with_capacity(blocks.len());
    let mut seen_user: Option<String> = None;
    for b in blocks.drain(..) {
        match &b {
            Block::TurnEnd { .. } => {
                seen_user = None;
                out.push(b);
            }
            Block::User { text, .. } => {
                if seen_user.as_deref() == Some(text.as_str()) {
                    continue;
                }
                seen_user = Some(text.clone());
                out.push(b);
            }
            _ => out.push(b),
        }
    }
    *blocks = out;
}

#[must_use]
pub fn blocks_to_markdown(blocks: &[Block]) -> String {
    let mut out = String::new();
    let mut tools_open = false;
    for block in blocks {
        match block {
            Block::User { text, .. } => {
                tools_open = false;
                out.push_str("## User\n");
                out.push_str(text);
                out.push_str("\n\n");
            }
            Block::Thought { text, .. } => {
                tools_open = false;
                out.push_str("## Thought\n");
                out.push_str(text);
                out.push_str("\n\n");
            }
            Block::Assistant { text, .. } => {
                tools_open = false;
                out.push_str("## Assistant\n");
                out.push_str(text);
                out.push_str("\n\n");
            }
            Block::Tool {
                title,
                input_preview,
                ..
            } => {
                if !tools_open {
                    out.push_str("## Tools\n");
                    tools_open = true;
                }
                out.push_str("- `");
                out.push_str(title);
                out.push_str("`: ");
                out.push_str(input_preview);
                out.push('\n');
            }
            Block::TurnEnd { .. } => {
                tools_open = false;
                out.push('\n');
            }
        }
    }
    out
}
