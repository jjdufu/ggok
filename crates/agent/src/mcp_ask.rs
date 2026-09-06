use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const TOOL_NAME: &str = "ask_user_question";
const WAIT_SECS: u64 = 1800;

/// Stdio MCP server grok calls for `WebUI` question cards.
///
/// # Errors
/// Returns an error if `stdin`/`stdout` fail or the daemon HTTP call fails.
pub fn run_mcp_ask() -> Result<()> {
    let url = std::env::var("GGOK_ASK_URL").context("GGOK_ASK_URL")?;
    let token = std::env::var("GGOK_ASK_TOKEN").context("GGOK_ASK_TOKEN")?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let lines = stdin.lock().lines();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = serde_json::from_str(&line)?;
        if msg.get("method").is_none() {
            continue;
        }
        if let Some(resp) = handle(&msg, &url, &token)? {
            stdout.write_all(serde_json::to_string(&resp)?.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn handle(msg: &Value, url: &str, token: &str) -> Result<Option<Value>> {
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let id = msg.get("id").cloned();
    let Some(id) = id else {
        return Ok(None);
    };
    let result = match method {
        "initialize" => json!({
            "protocolVersion": msg
                .pointer("/params/protocolVersion")
                .cloned()
                .unwrap_or_else(|| json!("2024-11-05")),
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "ggok-ask",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
        "ping" | "notifications/initialized" => {
            if method == "notifications/initialized" {
                return Ok(None);
            }
            json!({})
        }
        "tools/list" => json!({ "tools": [tool_schema()] }),
        "tools/call" => {
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            tool_call(&params, url, token)?
        }
        other => {
            return Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {other}") }
            })));
        }
    };
    Ok(Some(
        json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    ))
}

fn tool_schema() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Ask the user one or more multiple-choice questions. \
            Every question automatically gets an Other choice where the user can type their own answer. \
            Put your recommended option first and append (Recommended) to its label.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": { "type": "string" },
                            "header": { "type": "string" },
                            "multiSelect": { "type": "boolean" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": { "type": "string" },
                                        "description": { "type": "string" },
                                        "preview": { "type": "string" }
                                    },
                                    "required": ["label"]
                                }
                            }
                        },
                        "required": ["question", "options"]
                    }
                }
            },
            "required": ["questions"]
        }
    })
}

fn tool_call(params: &Value, base: &str, token: &str) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(TOOL_NAME);
    if name != TOOL_NAME {
        bail!("unknown tool {name}");
    }
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| params.clone());
    let questions = args
        .get("questions")
        .cloned()
        .unwrap_or_else(|| args.clone());
    let created = http_json(
        "POST",
        &format!("{}/api/ask", base.trim_end_matches('/')),
        token,
        Some(&json!({ "questions": questions })),
        Duration::from_secs(30),
    )?;
    let req = created
        .get("req")
        .and_then(Value::as_str)
        .context("ask response missing req")?
        .to_string();
    let reply = http_json(
        "GET",
        &format!("{}/api/ask/{}", base.trim_end_matches('/'), req),
        token,
        None,
        Duration::from_secs(WAIT_SECS),
    )?;
    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&reply)? }]
    }))
}

fn http_json(
    method: &str,
    url: &str,
    token: &str,
    body: Option<&Value>,
    timeout: Duration,
) -> Result<Value> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .context("GGOK_ASK_URL must be http")?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let path = format!("/{}", path.trim_start_matches('/'));
    let (host, port) = match hostport.split_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(80)),
        None => (hostport, 80),
    };
    let payload = body
        .map(serde_json::to_vec)
        .transpose()?
        .unwrap_or_default();
    let mut stream =
        TcpStream::connect((host, port)).with_context(|| format!("connect {hostport}"))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {hostport}\r\nAuthorization: Bearer {token}\r\nAccept: application/json\r\nConnection: close\r\n"
    );
    if !payload.is_empty() {
        req.push_str("Content-Type: application/json\r\n");
        req.push_str("Content-Length: ");
        req.push_str(&payload.len().to_string());
        req.push_str("\r\n");
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes())?;
    if !payload.is_empty() {
        stream.write_all(&payload)?;
    }
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    let code = status
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
    }
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    if !(200..300).contains(&code) {
        bail!(
            "ggok ask {code}: {}",
            buf.chars().take(300).collect::<String>()
        );
    }
    if buf.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&buf).context("ask json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_ask_tool() {
        let msg = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
        let resp = handle(&msg, "http://127.0.0.1:9", "t").unwrap().unwrap();
        assert_eq!(resp["result"]["tools"][0]["name"], json!(TOOL_NAME));
    }
}
