use super::rpc::{json_id_key, write_stdin};
use super::{Agent, live_entry};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AskOption {
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AskQuestion {
    pub question: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub header: String,
    #[serde(default)]
    pub options: Vec<AskOption>,
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestionView {
    pub req: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tool_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mode: String,
    pub questions: Vec<AskQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionReply {
    pub outcome: String,
    #[serde(default)]
    pub answers: Value,
    #[serde(default)]
    pub notes: Value,
}

#[derive(Debug, Clone)]
pub struct AskBridge {
    pub exe: PathBuf,
    pub url: String,
    pub token: String,
}

impl AskBridge {
    #[must_use]
    pub fn loopback_url(bind: &str) -> String {
        let port = bind.rsplit(':').next().unwrap_or("9888");
        format!("http://127.0.0.1:{port}")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PendingQuestion {
    pub(crate) rpc_id: Value,
    pub(crate) tool_id: String,
    pub(crate) mode: String,
    pub(crate) questions: Vec<AskQuestion>,
}

impl PendingQuestion {
    fn view(&self, req: &str) -> QuestionView {
        QuestionView {
            req: req.to_string(),
            tool_id: self.tool_id.clone(),
            mode: self.mode.clone(),
            questions: self.questions.clone(),
        }
    }
}

#[derive(Debug)]
struct ParsedAsk {
    session_id: String,
    tool_id: String,
    mode: String,
    questions: Vec<AskQuestion>,
}

#[must_use]
pub(crate) fn is_ask_user_method(method: &str) -> bool {
    let m = method.strip_prefix('_').unwrap_or(method);
    m == "x.ai/ask_user_question"
        || m == "ask_user_question"
        || m.ends_with("/ask_user_question")
        || m == "elicitation/create"
        || m.ends_with("/elicitation/create")
        || m == "x.ai/mcp/elicit"
        || m.ends_with("/mcp/elicit")
}

/// ACP `clientInfo.name`. Must not be `grok-pager`: that process intercepts
/// question cards as a local TUI view and never forwards them over stdio.
pub(crate) const ACP_CLIENT_IDENTIFIER: &str = "ggok";

pub(crate) const ASK_MCP_NAME: &str = "ggok-ask";
pub(crate) const ASK_MCP_TOOL: &str = "ask_user_question";

fn mentions_mcp_ask(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    lower == ASK_MCP_NAME
        || lower.starts_with("ggok-ask__")
        || lower.starts_with("ggok-ask/")
        || lower.contains("ggok-ask__")
        || lower.contains("ggok-ask/")
}

/// MCP `ggok-ask` cards are presented over HTTP; ACP `tool_call` envelopes must not.
#[must_use]
pub(crate) fn is_mcp_ask_tool(title: &str, name: &str) -> bool {
    mentions_mcp_ask(title) || mentions_mcp_ask(name)
}

/// Whether a session `tool_call` update should open a question card.
#[must_use]
pub(crate) fn should_present_tool_call_as_ask(title: &str, name: &str, update: &Value) -> bool {
    !is_mcp_ask_tool(title, name) && looks_like_ask_user(update)
}

#[must_use]
pub(crate) fn with_fallback_tool_id(mut params: Value, fallback: &str) -> Value {
    if fallback.is_empty() {
        return params;
    }
    if !parse_ask_user_params(&params).tool_id.is_empty() {
        return params;
    }
    if let Some(obj) = params.as_object_mut() {
        obj.insert("toolCallId".into(), json!(fallback));
    }
    params
}

fn client_ext_meta() -> Value {
    json!({
        "x.ai/incrementalBashOutput": true,
        "x.ai/bashOutputNoColor": true
    })
}

fn ask_user_rules() -> String {
    format!(
        "To ask the user a multiple-choice question, call use_tool on MCP server \
         {ASK_MCP_NAME}, tool {ASK_MCP_TOOL}, with a questions array. \
         Each item needs question text and options with label fields. Set multiSelect \
         true when several answers are allowed. Do not write the questions as markdown. \
         Do not say the tool is unavailable."
    )
}

/// `_meta` on `session/new` and `session/load`.
#[must_use]
pub(crate) fn acp_session_meta(permission_mode: &str) -> Value {
    let mut meta = Map::new();
    meta.insert("x.ai/incrementalBashOutput".into(), json!(true));
    meta.insert("x.ai/bashOutputNoColor".into(), json!(true));
    meta.insert("clientIdentifier".into(), json!(ACP_CLIENT_IDENTIFIER));
    meta.insert("clientVersion".into(), json!(env!("CARGO_PKG_VERSION")));
    meta.insert("rules".into(), json!(ask_user_rules()));
    match permission_mode {
        "always-approve" => {
            meta.insert("yoloMode".into(), json!(true));
        }
        "auto" => {
            meta.insert("autoMode".into(), json!(true));
        }
        _ => {}
    }
    Value::Object(meta)
}

/// `initialize` params for the ACP stdio client.
#[must_use]
pub(crate) fn acp_initialize_params() -> Value {
    let ext = client_ext_meta();
    json!({
        "protocolVersion": 1,
        "clientInfo": {
            "name": ACP_CLIENT_IDENTIFIER,
            "title": "GGOK",
            "version": env!("CARGO_PKG_VERSION")
        },
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false,
            "elicitation": { "form": {} },
            "_meta": ext.clone()
        },
        "capabilities": { "meta": ext },
        "_meta": {
            "clientIdentifier": ACP_CLIENT_IDENTIFIER,
            "clientVersion": env!("CARGO_PKG_VERSION")
        }
    })
}

#[must_use]
pub(crate) fn mcp_ask_servers(bridge: &AskBridge) -> Value {
    json!([{
        "name": ASK_MCP_NAME,
        "command": bridge.exe,
        "args": ["__mcp-ask"],
        "env": [
            {"name": "GGOK_ASK_URL", "value": bridge.url},
            {"name": "GGOK_ASK_TOKEN", "value": bridge.token}
        ]
    }])
}

#[must_use]
pub(crate) fn looks_like_ask_user(params: &Value) -> bool {
    !parse_ask_user_params(params).questions.is_empty()
}

/// Unwrap ACP `ext_method` envelopes so the inner `_x.ai/ask_user_question` payload
/// is visible to [`parse_ask_user_params`].
#[must_use]
pub(crate) fn unwrap_ext_request(method: &str, params: Value) -> (String, Value) {
    if is_ask_user_method(method) {
        return (method.to_string(), params);
    }
    let stripped = method.strip_prefix('_').unwrap_or(method);
    let nested_method = json_str(&params, &["method", "name"]);
    let wrapped = stripped.is_empty()
        || stripped == "_"
        || stripped == "ext_method"
        || stripped == "session/ext_method"
        || method == "_";
    if wrapped && is_ask_user_method(&nested_method) {
        let inner = params
            .get("params")
            .cloned()
            .unwrap_or_else(|| params.clone());
        return (nested_method, merge_session_fields(&params, inner));
    }
    (method.to_string(), params)
}

fn merge_session_fields(outer: &Value, mut inner: Value) -> Value {
    if let Some(obj) = inner.as_object_mut() {
        for key in [
            "sessionId",
            "session_id",
            "toolCallId",
            "tool_call_id",
            "mode",
            "questions",
        ] {
            let missing = match obj.get(key) {
                None | Some(Value::Null) => true,
                Some(Value::String(s)) => s.is_empty(),
                Some(Value::Array(a)) => a.is_empty(),
                _ => false,
            };
            if missing && let Some(v) = outer.get(key).cloned() {
                obj.insert((*key).into(), v);
            }
        }
    }
    inner
}

fn json_str(v: &Value, keys: &[&str]) -> String {
    for key in keys {
        match v.get(*key) {
            Some(Value::String(s)) if !s.trim().is_empty() => return s.trim().to_string(),
            Some(Value::Number(n)) => return n.to_string(),
            _ => {}
        }
    }
    String::new()
}

fn json_bool(v: &Value, keys: &[&str]) -> bool {
    keys.iter()
        .find_map(|key| v.get(*key).and_then(Value::as_bool))
        .unwrap_or(false)
}

fn short_header(question: &str) -> String {
    let mut out = String::new();
    for ch in question.chars() {
        if out.chars().count() >= 12 {
            break;
        }
        out.push(ch);
    }
    out
}

fn parse_option(v: &Value) -> Option<AskOption> {
    match v {
        Value::String(s) => {
            let label = s.trim();
            if label.is_empty() {
                return None;
            }
            return Some(AskOption {
                label: label.to_string(),
                description: String::new(),
                preview: None,
            });
        }
        Value::Number(n) => {
            return Some(AskOption {
                label: n.to_string(),
                description: String::new(),
                preview: None,
            });
        }
        Value::Object(_) => {}
        _ => return None,
    }
    let label = json_str(
        v,
        &["label", "name", "title", "text", "value", "const", "id"],
    );
    if label.is_empty() {
        return None;
    }
    let description = json_str(v, &["description", "desc", "detail"]);
    let preview = {
        let s = json_str(v, &["preview"]);
        if s.is_empty() { None } else { Some(s) }
    };
    Some(AskOption {
        label,
        description,
        preview,
    })
}

fn parse_options(v: &Value) -> Vec<AskOption> {
    for key in ["options", "choices", "items"] {
        if let Some(arr) = v.get(key).and_then(Value::as_array) {
            let opts: Vec<AskOption> = arr.iter().filter_map(parse_option).collect();
            if !opts.is_empty() {
                return opts;
            }
        }
        if let Some(obj) = v.get(key).and_then(Value::as_object) {
            let opts: Vec<AskOption> = obj
                .iter()
                .filter_map(|(label, val)| {
                    let label = label.trim();
                    if label.is_empty() {
                        return None;
                    }
                    let description = match val {
                        Value::String(s) => s.trim().to_string(),
                        Value::Object(_) => json_str(val, &["description", "desc", "detail"]),
                        _ => String::new(),
                    };
                    let preview = match val {
                        Value::Object(_) => {
                            let s = json_str(val, &["preview"]);
                            if s.is_empty() { None } else { Some(s) }
                        }
                        _ => None,
                    };
                    Some(AskOption {
                        label: label.to_string(),
                        description,
                        preview,
                    })
                })
                .collect();
            if !opts.is_empty() {
                return opts;
            }
        }
    }
    for key in ["oneOf", "anyOf", "enum"] {
        if let Some(arr) = v.get(key).and_then(Value::as_array) {
            let opts: Vec<AskOption> = arr.iter().filter_map(parse_option).collect();
            if !opts.is_empty() {
                return opts;
            }
        }
    }
    Vec::new()
}

fn parse_one_question(v: &Value) -> Option<AskQuestion> {
    if let Value::String(s) = v {
        let question = s.trim();
        if question.is_empty() {
            return None;
        }
        return Some(AskQuestion {
            question: question.to_string(),
            header: short_header(question),
            options: Vec::new(),
            multi_select: false,
        });
    }
    let mut question = json_str(v, &["question", "prompt", "text", "message"]);
    if question.is_empty() {
        question = json_str(v, &["header"]);
    }
    if question.is_empty() && looks_like_question_object(v) {
        question = json_str(v, &["title"]);
    }
    let options = parse_options(v);
    if question.is_empty() && options.is_empty() {
        return None;
    }
    if question.is_empty() {
        question = "Question".into();
    }
    let mut header = json_str(v, &["header", "title"]);
    if header.is_empty() {
        header = short_header(&question);
    }
    Some(AskQuestion {
        question,
        header,
        options,
        multi_select: json_bool(v, &["multi_select", "multiSelect", "allow_multiple"]),
    })
}

fn looks_like_question_object(v: &Value) -> bool {
    let Some(obj) = v.as_object() else {
        return false;
    };
    obj.contains_key("question")
        || obj.contains_key("prompt")
        || obj.contains_key("options")
        || obj.contains_key("choices")
        || obj.contains_key("oneOf")
        || obj.contains_key("anyOf")
        || obj.contains_key("enum")
}

fn questions_from(v: &Value) -> Vec<AskQuestion> {
    if let Some(arr) = v.get("questions").and_then(Value::as_array) {
        let parsed: Vec<AskQuestion> = arr.iter().filter_map(parse_one_question).collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    if let Some(arr) = v.get("items").and_then(Value::as_array) {
        let parsed: Vec<AskQuestion> = arr
            .iter()
            .filter(|item| looks_like_question_object(item))
            .filter_map(parse_one_question)
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    if let Some(obj) = v.get("questions").filter(|q| q.is_object())
        && let Some(q) = parse_one_question(obj)
    {
        return vec![q];
    }
    let schema_qs = questions_from_schema(v.get("requestedSchema").unwrap_or(v));
    if !schema_qs.is_empty() {
        return schema_qs;
    }
    if looks_like_question_object(v)
        && let Some(q) = parse_one_question(v)
    {
        return vec![q];
    }
    Vec::new()
}

fn questions_from_schema(schema: &Value) -> Vec<AskQuestion> {
    let Some(props) = schema.get("properties").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, prop) in props {
        let mut q = if let Some(q) = parse_one_question(prop) {
            q
        } else {
            let options = parse_options(prop);
            if options.is_empty() && json_str(prop, &["title", "description"]).is_empty() {
                continue;
            }
            AskQuestion {
                question: json_str(prop, &["title", "description"]),
                header: String::new(),
                options,
                multi_select: json_bool(
                    prop,
                    &["multi_select", "multiSelect", "allow_multiple"],
                ),
            }
        };
        if q.question.is_empty() {
            q.question = json_str(prop, &["title"]);
            if q.question.is_empty() {
                q.question.clone_from(key);
            }
        }
        q.header.clone_from(key);
        if q.options.is_empty() {
            q.options = parse_options(prop);
        }
        out.push(q);
    }
    out
}

fn push_value_layers<'a>(v: &'a Value, out: &mut Vec<&'a Value>, depth: usize) {
    if depth > 4 {
        return;
    }
    out.push(v);
    let Some(obj) = v.as_object() else {
        return;
    };
    for key in [
        "params",
        "request",
        "data",
        "input",
        "arguments",
        "rawInput",
        "payload",
        "body",
        "toolCall",
    ] {
        if let Some(child) = obj.get(key) {
            push_value_layers(child, out, depth + 1);
        }
    }
}

fn value_layers(root: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    push_value_layers(root, &mut out, 0);
    out
}

fn parse_ask_user_params(params: &Value) -> ParsedAsk {
    let layers = value_layers(params);
    let session_id = layers
        .iter()
        .map(|v| json_str(v, &["sessionId", "session_id"]))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    let tool_id = layers
        .iter()
        .map(|v| json_str(v, &["toolCallId", "tool_call_id"]))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    let mode = layers
        .iter()
        .map(|v| json_str(v, &["mode"]))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    let questions = layers
        .iter()
        .map(|v| questions_from(v))
        .find(|q| !q.is_empty())
        .unwrap_or_default();
    ParsedAsk {
        session_id,
        tool_id,
        mode,
        questions,
    }
}

fn normalize_outcome(raw: &str) -> &str {
    match raw.trim() {
        "skip" | "skip_interview" | "cancelled" | "canceled" => "skip_interview",
        "chat" | "chat_about_this" => "chat_about_this",
        _ => "accepted",
    }
}

fn elicitation_result(
    outcome: &str,
    questions: &[AskQuestion],
    answers: &Value,
    notes: &Value,
) -> Result<Value> {
    if normalize_outcome(outcome) != "accepted" {
        return Ok(json!({ "action": "cancel" }));
    }
    let (validated, _) = validate_accepted(questions, answers, notes)?;
    let mut content = Map::new();
    for q in questions {
        let key = if q.header.is_empty() {
            q.question.clone()
        } else {
            q.header.clone()
        };
        if let Some(v) = validated.get(&q.question) {
            content.insert(key, v.clone());
        }
    }
    Ok(json!({ "action": "accept", "content": content }))
}

fn ask_user_result(outcome: &str, answers: Value, annotations: Value) -> Value {
    match normalize_outcome(outcome) {
        "skip_interview" => json!({ "outcome": "skip_interview" }),
        "chat_about_this" => json!({ "outcome": "chat_about_this" }),
        _ => {
            let mut body = Map::new();
            body.insert("outcome".into(), json!("accepted"));
            body.insert("answers".into(), answers);
            body.insert("partial_answers".into(), json!({}));
            if annotations.as_object().is_some_and(|obj| !obj.is_empty()) {
                body.insert("annotations".into(), annotations);
            }
            Value::Object(body)
        }
    }
}

fn jsonrpc_result(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn preview_for(question: &AskQuestion, label: &str) -> Option<String> {
    question
        .options
        .iter()
        .find(|o| o.label == label)
        .and_then(|o| o.preview.clone())
}

fn note_for(notes: &Value, question: &str) -> String {
    notes
        .get(question)
        .and_then(Value::as_str)
        .map_or("", str::trim)
        .to_string()
}

fn push_string_list(out: &mut Vec<String>, v: &Value) -> Result<()> {
    match v {
        Value::String(s) => {
            let t = s.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
            Ok(())
        }
        Value::Array(arr) => {
            for item in arr {
                let Some(s) = item.as_str().map(str::trim) else {
                    bail!("answer list must be strings");
                };
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            }
            Ok(())
        }
        Value::Null => Ok(()),
        _ => bail!("answer must be a string or list of strings"),
    }
}

fn validate_accepted(
    questions: &[AskQuestion],
    answers: &Value,
    notes: &Value,
) -> Result<(Value, Value)> {
    let obj = answers.as_object();
    let mut out_answers = Map::new();
    let mut annotations = Map::new();
    for q in questions {
        let raw = obj.and_then(|m| m.get(&q.question)).unwrap_or(&Value::Null);
        let mut picked = Vec::new();
        push_string_list(&mut picked, raw)?;
        let note = note_for(notes, &q.question);
        if picked.is_empty() && note.is_empty() {
            bail!("missing answer for question");
        }
        if !q.multi_select && picked.len() > 1 {
            bail!("question does not allow multiple answers");
        }
        if picked.is_empty() {
            picked.push(note.clone());
        }
        let answer_val = if q.multi_select {
            json!(picked)
        } else {
            json!(picked[0])
        };
        let preview = if q.multi_select {
            None
        } else {
            preview_for(q, &picked[0])
        };
        if preview.is_some() || !note.is_empty() {
            let mut ann = Map::new();
            if let Some(p) = preview {
                ann.insert("preview".into(), json!(p));
            }
            if !note.is_empty() {
                ann.insert("notes".into(), json!(note));
            }
            annotations.insert(q.question.clone(), Value::Object(ann));
        }
        out_answers.insert(q.question.clone(), answer_val);
    }
    Ok((Value::Object(out_answers), Value::Object(annotations)))
}

pub(crate) fn question_views(questions: &HashMap<String, PendingQuestion>) -> Vec<QuestionView> {
    let mut rows: Vec<QuestionView> = questions
        .iter()
        .map(|(req, pending)| pending.view(req))
        .collect();
    rows.sort_by(|a, b| a.req.cmp(&b.req));
    rows
}

impl Agent {
    #[must_use]
    pub async fn pending_questions(&self, id: &str) -> Vec<QuestionView> {
        let g = self.inner.lock().await;
        g.sessions
            .get(id)
            .map(|sess| question_views(&sess.questions))
            .unwrap_or_default()
    }

    pub(crate) async fn clear_questions(&self, id: &str) {
        let empty = {
            let mut g = self.inner.lock().await;
            let reqs = {
                let Some(sess) = g.sessions.get_mut(id) else {
                    return;
                };
                if sess.questions.is_empty() {
                    return;
                }
                let reqs: Vec<String> = sess.questions.keys().cloned().collect();
                sess.questions.clear();
                reqs
            };
            let skip = QuestionReply {
                outcome: "skip_interview".into(),
                answers: Value::Null,
                notes: Value::Null,
            };
            for req in reqs {
                if let Some(tx) = g.question_tx.remove(&req) {
                    let _ = tx.send(skip.clone());
                }
            }
            true
        };
        if empty {
            self.emit(id, "questions", &Vec::<QuestionView>::new());
        }
    }

    pub(crate) async fn handle_ask_user(&self, rpc_id: Value, params: Value) {
        let mut parsed = parse_ask_user_params(&params);
        if parsed.session_id.is_empty() {
            parsed.session_id = self
                .inner
                .lock()
                .await
                .web_active_id
                .clone()
                .unwrap_or_default();
        }
        if parsed.session_id.is_empty() || parsed.questions.is_empty() {
            tracing::warn!(
                session_id = %parsed.session_id,
                question_count = parsed.questions.len(),
                params = %params,
                "ask_user_question skipped (missing session id or questions)"
            );
            if !rpc_id.is_null() {
                let result = ask_user_result("skip_interview", Value::Null, Value::Null);
                let msg = jsonrpc_result(&rpc_id, &result);
                let mut g = self.inner.lock().await;
                let _ = write_stdin(g.stdin.as_mut(), &msg).await;
            }
            return;
        }
        tracing::info!(
            session_id = %parsed.session_id,
            question_count = parsed.questions.len(),
            option_count = parsed
                .questions
                .iter()
                .map(|q| q.options.len())
                .sum::<usize>(),
            "ask_user_question pending"
        );
        let req_key = if !parsed.tool_id.is_empty() {
            parsed.tool_id.clone()
        } else if !rpc_id.is_null() {
            json_id_key(&rpc_id)
        } else {
            Uuid::new_v4().to_string()
        };
        let views = {
            let mut g = self.inner.lock().await;
            let sess = live_entry(&mut g, &parsed.session_id, "");
            let rpc_id = match sess.questions.get(&req_key) {
                Some(prev) if rpc_id.is_null() && !prev.rpc_id.is_null() => prev.rpc_id.clone(),
                _ => rpc_id,
            };
            sess.questions.insert(
                req_key.clone(),
                PendingQuestion {
                    rpc_id,
                    tool_id: parsed.tool_id,
                    mode: parsed.mode,
                    questions: parsed.questions,
                },
            );
            question_views(&sess.questions)
        };
        self.emit(&parsed.session_id, "questions", &views);
    }

    /// # Errors
    /// Returns an error if the session is not attached, the question is missing,
    /// answers are invalid, or stdin write fails.
    pub async fn answer_question(&self, id: &str, req: &str, reply: QuestionReply) -> Result<()> {
        self.require_attached(id).await?;
        let mut g = self.inner.lock().await;
        let pending = {
            let Some(sess) = g.sessions.get_mut(id) else {
                bail!("session not loaded");
            };
            let Some(pending) = sess.questions.get(req).cloned() else {
                bail!("question request not found");
            };
            sess.questions.remove(req);
            pending
        };
        let outcome = normalize_outcome(&reply.outcome);
        let result = if pending.mode == "form" {
            elicitation_result(outcome, &pending.questions, &reply.answers, &reply.notes)?
        } else if outcome == "accepted" {
            let (answers, annotations) =
                validate_accepted(&pending.questions, &reply.answers, &reply.notes)?;
            ask_user_result(outcome, answers, annotations)
        } else {
            ask_user_result(outcome, Value::Null, Value::Null)
        };
        if let Some(tx) = g.question_tx.remove(req) {
            let _ = tx.send(reply);
        }
        if !pending.rpc_id.is_null() {
            let msg = jsonrpc_result(&pending.rpc_id, &result);
            if let Err(e) = write_stdin(g.stdin.as_mut(), &msg).await {
                if let Some(sess) = g.sessions.get_mut(id) {
                    sess.questions.insert(req.to_string(), pending);
                }
                return Err(e);
            }
        }
        let views = g
            .sessions
            .get(id)
            .map(|sess| question_views(&sess.questions))
            .unwrap_or_default();
        drop(g);
        self.emit(id, "questions", &views);
        Ok(())
    }

    /// # Errors
    /// Returns an error if no session is active or the payload has no questions.
    pub async fn present_web_question(
        &self,
        session_id: Option<&str>,
        questions: Value,
    ) -> Result<(String, String)> {
        let mut sid = session_id
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        if sid.is_empty() {
            sid = self
                .inner
                .lock()
                .await
                .web_active_id
                .clone()
                .unwrap_or_default();
        }
        let mut params = json!({ "questions": questions });
        if sid.is_empty() {
            bail!("no active session");
        }
        if let Some(obj) = params.as_object_mut() {
            obj.insert("sessionId".into(), json!(sid));
        }
        let req = Uuid::new_v4().to_string();
        if let Some(obj) = params.as_object_mut() {
            obj.insert("toolCallId".into(), json!(req));
        }
        let (tx, rx) = oneshot::channel();
        {
            let mut g = self.inner.lock().await;
            g.question_tx.insert(req.clone(), tx);
            g.question_rx.insert(req.clone(), rx);
        }
        self.handle_ask_user(Value::Null, params).await;
        let found = {
            let g = self.inner.lock().await;
            g.sessions
                .get(&sid)
                .is_some_and(|sess| sess.questions.contains_key(&req))
        };
        if !found {
            let mut g = self.inner.lock().await;
            g.question_tx.remove(&req);
            g.question_rx.remove(&req);
            bail!("ask_user_question skipped (missing session id or questions)");
        }
        Ok((sid, req))
    }

    /// # Errors
    /// Returns an error if the question is missing or the waiter was dropped.
    pub async fn wait_web_question(&self, req: &str) -> Result<QuestionReply> {
        let rx = {
            let mut g = self.inner.lock().await;
            g.question_rx
                .remove(req)
                .ok_or_else(|| anyhow::anyhow!("question request not found"))?
        };
        rx.await
            .map_err(|_| anyhow::anyhow!("question waiter closed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(question: &str, labels: &[&str], multi: bool) -> AskQuestion {
        AskQuestion {
            question: question.into(),
            header: "H".into(),
            options: labels
                .iter()
                .map(|label| AskOption {
                    label: (*label).into(),
                    description: String::new(),
                    preview: None,
                })
                .collect(),
            multi_select: multi,
        }
    }

    #[test]
    fn detects_ext_method_names() {
        assert!(is_ask_user_method("_x.ai/ask_user_question"));
        assert!(is_ask_user_method("x.ai/ask_user_question"));
        assert!(is_ask_user_method("ask_user_question"));
        assert!(is_ask_user_method("_foo/ask_user_question"));
        assert!(is_ask_user_method("elicitation/create"));
        assert!(is_ask_user_method("x.ai/mcp/elicit"));
        assert!(!is_ask_user_method("session/request_permission"));
        assert!(!is_ask_user_method("ext_method"));
    }

    #[test]
    fn unwraps_ext_method_envelope() {
        let params = json!({
            "method": "x.ai/ask_user_question",
            "sessionId": "sid",
            "params": {
                "toolCallId": "call-1",
                "questions": [{ "question": "Pick?", "options": ["A"] }]
            }
        });
        let (method, inner) = unwrap_ext_request("ext_method", params);
        assert!(is_ask_user_method(&method));
        let parsed = parse_ask_user_params(&inner);
        assert_eq!(parsed.session_id, "sid");
        assert_eq!(parsed.tool_id, "call-1");
        assert_eq!(parsed.questions[0].options[0].label, "A");
    }

    #[test]
    fn parses_flat_params() {
        let params = json!({
            "sessionId": "sid",
            "toolCallId": "call-1",
            "mode": "default",
            "questions": [{
                "question": "Which library?",
                "header": "Lib",
                "multiSelect": true,
                "options": [
                    {"label": "Day.js", "description": "small", "preview": "`dayjs()`"},
                    {"name": "date-fns"}
                ]
            }]
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.session_id, "sid");
        assert_eq!(parsed.tool_id, "call-1");
        assert_eq!(parsed.mode, "default");
        assert_eq!(parsed.questions.len(), 1);
        assert!(parsed.questions[0].multi_select);
        assert_eq!(parsed.questions[0].header, "Lib");
        assert_eq!(
            parsed.questions[0].options[0].preview.as_deref(),
            Some("`dayjs()`")
        );
        assert_eq!(parsed.questions[0].options[1].label, "date-fns");
    }

    #[test]
    fn parses_nested_tool_call_and_snake_case() {
        let params = json!({
            "session_id": "s",
            "toolCall": {"toolCallId": "t"},
            "questions": [{
                "prompt": "Go?",
                "multi_select": false,
                "options": [{"label": "Yes"}]
            }]
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.session_id, "s");
        assert_eq!(parsed.tool_id, "t");
        assert_eq!(parsed.questions[0].question, "Go?");
        assert!(!parsed.questions[0].multi_select);
        assert_eq!(parsed.questions[0].header, "Go?");
    }

    #[test]
    fn keeps_options_without_question_text() {
        let params = json!({
            "sessionId": "s",
            "questions": [{ "options": [{"label": "A"}] }]
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].options[0].label, "A");
    }

    #[test]
    fn parses_header_and_string_questions() {
        let params = json!({
            "sessionId": "s",
            "questions": ["Bare?", { "header": "From header", "options": ["Yes"] }]
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.questions[0].question, "Bare?");
        assert_eq!(parsed.questions[1].question, "From header");
        assert_eq!(parsed.questions[1].options[0].label, "Yes");
    }

    #[test]
    fn session_meta_points_at_mcp_ask() {
        let yolo = acp_session_meta("always-approve");
        assert_eq!(yolo["yoloMode"], json!(true));
        let rules = yolo["rules"].as_str().unwrap_or("");
        assert!(rules.contains(ASK_MCP_NAME));
        assert!(rules.contains(ASK_MCP_TOOL));
        assert!(rules.contains("use_tool"));
        assert!(!rules.contains("search_tool"));
        let ask = acp_session_meta("ask");
        assert!(ask.get("yoloMode").is_none());
        let init = acp_initialize_params();
        assert_eq!(
            init["_meta"]["clientIdentifier"],
            json!(ACP_CLIENT_IDENTIFIER)
        );
        assert_eq!(init["clientCapabilities"]["elicitation"]["form"], json!({}));
        assert_eq!(init["clientInfo"]["name"], json!("ggok"));
        let servers = mcp_ask_servers(&AskBridge {
            exe: PathBuf::from("/bin/ggok"),
            url: "http://127.0.0.1:9888".into(),
            token: "t".into(),
        });
        assert_eq!(servers[0]["name"], json!(ASK_MCP_NAME));
        assert_eq!(servers[0]["args"][0], json!("__mcp-ask"));
    }

    #[test]
    fn parses_elicitation_schema() {
        let params = json!({
            "sessionId": "s",
            "mode": "form",
            "message": "How to proceed?",
            "requestedSchema": {
                "type": "object",
                "properties": {
                    "strategy": {
                        "title": "Refactoring Strategy",
                        "oneOf": [
                            {"const": "A", "title": "A", "description": "first"},
                            {"const": "B"}
                        ]
                    }
                }
            }
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.session_id, "s");
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].question, "Refactoring Strategy");
        assert_eq!(parsed.questions[0].header, "strategy");
        assert_eq!(parsed.questions[0].options[0].label, "A");
    }

    #[test]
    fn looks_like_nested_questions() {
        assert!(looks_like_ask_user(&json!({
            "params": { "questions": [{ "question": "Go?", "options": ["Yes"] }] }
        })));
        assert!(!looks_like_ask_user(&json!({ "foo": 1 })));
    }

    #[test]
    fn ignores_search_tools_and_mcp_tool_titles() {
        assert!(!looks_like_ask_user(&json!({
            "title": "Search tools: \"ggok-ask ask_user_question\"",
            "rawInput": { "query": "ggok-ask ask_user_question" }
        })));
        assert!(!looks_like_ask_user(&json!({
            "title": "ggok-ask__ask_user_question",
            "toolCallId": "t-mcp"
        })));
        assert!(!looks_like_ask_user(&json!({
            "sessionUpdate": "tool_call",
            "title": "ls -la",
            "rawInput": { "command": "ls -la" }
        })));
    }

    #[test]
    fn mcp_ask_tool_call_does_not_present_even_with_questions() {
        let update = json!({
            "title": "ggok-ask__ask_user_question",
            "toolCallId": "t-mcp",
            "rawInput": {
                "questions": [{ "question": "早饭?", "options": ["米饭", "馒头"] }]
            }
        });
        assert!(looks_like_ask_user(&update));
        assert!(is_mcp_ask_tool("ggok-ask__ask_user_question", "ask_user_question"));
        assert!(!should_present_tool_call_as_ask(
            "ggok-ask__ask_user_question",
            "ask_user_question",
            &update
        ));
        assert!(!is_mcp_ask_tool(
            "Search tools: \"ggok-ask ask_user_question\"",
            "search_tools"
        ));
        assert!(should_present_tool_call_as_ask(
            "ask_user_question",
            "ask_user_question",
            &update
        ));
    }

    #[test]
    fn fills_missing_tool_call_id() {
        let params = json!({
            "questions": [{ "question": "Go?", "options": ["Yes"] }]
        });
        let filled = with_fallback_tool_id(params, "last-1");
        assert_eq!(filled["toolCallId"], json!("last-1"));
        let keep = with_fallback_tool_id(
            json!({
                "toolCallId": "keep",
                "questions": [{ "question": "Go?", "options": ["Yes"] }]
            }),
            "last-1",
        );
        assert_eq!(keep["toolCallId"], json!("keep"));
    }

    #[test]
    fn parses_string_and_title_options() {
        let params = json!({
            "sessionId": "s",
            "questions": [{
                "question": "Pick?",
                "options": ["Alpha", {"title": "Beta", "description": "b"}]
            }]
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.questions[0].options.len(), 2);
        assert_eq!(parsed.questions[0].options[0].label, "Alpha");
        assert_eq!(parsed.questions[0].options[1].label, "Beta");
        assert_eq!(parsed.questions[0].options[1].description, "b");
    }

    #[test]
    fn parses_nested_params_and_single_question() {
        let params = json!({
            "sessionId": "s",
            "params": {
                "questions": {
                    "question": "Go?",
                    "choices": { "Yes": "do it", "No": "stop" }
                }
            }
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.session_id, "s");
        assert_eq!(parsed.questions[0].question, "Go?");
        let labels: Vec<_> = parsed.questions[0]
            .options
            .iter()
            .map(|o| o.label.as_str())
            .collect();
        assert!(labels.contains(&"Yes"));
        assert!(labels.contains(&"No"));
    }

    #[test]
    fn parses_tool_call_raw_input() {
        let params = json!({
            "sessionId": "s",
            "toolCall": {
                "toolCallId": "t1",
                "rawInput": {
                    "questions": [{
                        "question": "Which?",
                        "oneOf": [
                            {"const": "A", "title": "A", "description": "first"},
                            {"const": "B"}
                        ]
                    }]
                }
            }
        });
        let parsed = parse_ask_user_params(&params);
        assert_eq!(parsed.tool_id, "t1");
        assert_eq!(parsed.questions[0].options[0].label, "A");
        assert_eq!(parsed.questions[0].options[0].description, "first");
        assert_eq!(parsed.questions[0].options[1].label, "B");
    }

    #[test]
    fn result_shapes() {
        assert_eq!(
            ask_user_result("skip", Value::Null, Value::Null),
            json!({ "outcome": "skip_interview" })
        );
        assert_eq!(
            ask_user_result("cancelled", Value::Null, Value::Null),
            json!({ "outcome": "skip_interview" })
        );
        assert_eq!(
            ask_user_result("chat_about_this", Value::Null, Value::Null),
            json!({ "outcome": "chat_about_this" })
        );
        let accepted = ask_user_result(
            "accepted",
            json!({ "Which?": "A" }),
            json!({ "Which?": { "notes": "n", "preview": "p" } }),
        );
        assert_eq!(accepted["outcome"], "accepted");
        assert_eq!(accepted["answers"]["Which?"], "A");
        assert_eq!(accepted["partial_answers"], json!({}));
        assert_eq!(accepted["annotations"]["Which?"]["notes"], "n");
    }

    #[test]
    fn validates_single_and_multi() {
        let single = vec![q("Pick?", &["A", "B"], false)];
        let (answers, _) =
            validate_accepted(&single, &json!({ "Pick?": "A" }), &Value::Null).unwrap();
        assert_eq!(answers["Pick?"], "A");

        let multi = vec![q("Pick?", &["A", "B"], true)];
        let (answers, _) =
            validate_accepted(&multi, &json!({ "Pick?": ["A", "B"] }), &Value::Null).unwrap();
        assert_eq!(answers["Pick?"], json!(["A", "B"]));

        assert!(validate_accepted(&single, &json!({}), &Value::Null).is_err());
        assert!(validate_accepted(&single, &json!({ "Pick?": ["A", "B"] }), &Value::Null).is_err());
    }

    #[test]
    fn accepts_other_text_and_notes() {
        let qs = vec![AskQuestion {
            question: "Pick?".into(),
            header: "P".into(),
            options: vec![AskOption {
                label: "A".into(),
                description: String::new(),
                preview: Some("prev".into()),
            }],
            multi_select: false,
        }];
        let (answers, ann) = validate_accepted(
            &qs,
            &json!({ "Pick?": "A" }),
            &json!({ "Pick?": "because" }),
        )
        .unwrap();
        assert_eq!(answers["Pick?"], "A");
        assert_eq!(ann["Pick?"]["preview"], "prev");
        assert_eq!(ann["Pick?"]["notes"], "because");

        let (answers, _ann) =
            validate_accepted(&qs, &json!({ "Pick?": "custom" }), &Value::Null).unwrap();
        assert_eq!(answers["Pick?"], "custom");
    }

    #[test]
    fn other_note_only_counts_as_answer() {
        let qs = vec![q("Pick?", &["A"], false)];
        let (answers, _) =
            validate_accepted(&qs, &json!({}), &json!({ "Pick?": "typed" })).unwrap();
        assert_eq!(answers["Pick?"], "typed");
    }
}
