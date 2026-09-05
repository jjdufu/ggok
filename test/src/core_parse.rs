use ggok_core::parse::{
    Ingest, Parser, apply_event, blocks_to_markdown, fallback_window, merge_live_over_disk,
    models_from_cache, parse_updates_file, timestamp_ms,
};
use ggok_core::types::Block;
use serde_json::json;
use std::fs;

fn chunk(kind: &str, text: &str) -> serde_json::Value {
    json!({
        "sessionUpdate": kind,
        "content": { "text": text }
    })
}

#[test]
fn fallback_window_by_model_id() {
    assert_eq!(fallback_window("grok-4"), 500_000);
    assert_eq!(fallback_window("grok-4.5-build"), 500_000);
    assert_eq!(fallback_window("grok-4.6"), 500_000);
    assert_eq!(fallback_window("grok-3"), 200_000);
    assert_eq!(fallback_window("unknown"), 200_000);
}

#[test]
fn timestamp_ms_seconds_millis_and_rfc3339() {
    assert_eq!(
        timestamp_ms(Some(&json!(1_700_000_000))),
        Some(1_700_000_000_000)
    );
    assert_eq!(
        timestamp_ms(Some(&json!(1_700_000_000_000_u64))),
        Some(1_700_000_000_000)
    );
    assert!(timestamp_ms(Some(&json!("2024-01-15T00:00:00Z"))).is_some());
    assert!(timestamp_ms(None).is_none());
    assert!(timestamp_ms(Some(&json!(true))).is_none());
}

#[test]
fn ingest_merges_text_chunks_then_turn_end() {
    let mut p = Parser::new();
    assert_eq!(
        p.ingest_at(&chunk("user_message_chunk", "Hi"), "p1", Some(10)),
        Ingest::Text
    );
    assert_eq!(
        p.ingest_at(&chunk("agent_message_chunk", "Hel"), "p1", Some(20)),
        Ingest::Text
    );
    assert_eq!(
        p.ingest_at(&chunk("agent_message_chunk", "lo"), "p1", Some(30)),
        Ingest::Text
    );
    assert_eq!(p.work_started_ms(), Some(10));
    assert_eq!(
        p.ingest_at(
            &json!({
                "sessionUpdate": "turn_completed",
                "stop_reason": "end",
                "elapsed_ms": 40
            }),
            "p1",
            Some(50)
        ),
        Ingest::TurnEnd
    );

    let blocks = p.snapshot_blocks();
    assert!(
        matches!(&blocks[0], Block::User { text, prompt_id, .. } if text == "Hi" && prompt_id == "p1")
    );
    assert!(
        matches!(&blocks[1], Block::Assistant { text, prompt_id } if text == "Hello" && prompt_id == "p1")
    );
    assert!(matches!(
        &blocks[2],
        Block::TurnEnd {
            prompt_id,
            cancelled,
            duration_ms
        } if prompt_id == "p1" && !cancelled && *duration_ms == 40
    ));
}

#[test]
fn ingest_tool_and_usage() {
    let mut p = Parser::new();
    assert_eq!(
        p.ingest_at(
            &json!({
                "sessionUpdate": "tool_call",
                "toolCallId": "t1",
                "title": "read",
                "rawInput": { "path": "/tmp/a.rs" }
            }),
            "p1",
            Some(1)
        ),
        Ingest::Tool
    );
    assert_eq!(
        p.ingest_at(
            &json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "t1",
                "status": "completed"
            }),
            "p1",
            Some(2)
        ),
        Ingest::Tool
    );
    let tool = p.tool("t1").expect("tool");
    assert!(
        matches!(tool, Block::Tool { id, title, status, .. } if id == "t1" && title == "read" && status == "completed")
    );

    assert_eq!(
        p.ingest_at(
            &json!({ "usage": { "inputTokens": 3, "outputTokens": 5 } }),
            "",
            None
        ),
        Ingest::Usage
    );
}

#[test]
fn apply_event_reads_prompt_id_from_meta() {
    let mut p = Parser::new();
    apply_event(
        &mut p,
        &json!({
            "timestamp": 1_700_000_000,
            "params": {
                "_meta": { "promptId": "meta-1" },
                "update": {
                    "sessionUpdate": "user_message_chunk",
                    "content": { "text": "q" }
                }
            }
        }),
    );
    let blocks = p.snapshot_blocks();
    assert!(
        matches!(&blocks[0], Block::User { prompt_id, text, .. } if prompt_id == "meta-1" && text == "q")
    );
}

#[test]
fn merge_live_over_disk_appends_after_last_turn() {
    let disk = vec![
        Block::User {
            prompt_id: "old".into(),
            text: "a".into(),
            files: vec![],
        },
        Block::TurnEnd {
            prompt_id: "old".into(),
            duration_ms: 1,
            cancelled: false,
        },
        Block::User {
            prompt_id: "new".into(),
            text: "partial".into(),
            files: vec![],
        },
    ];
    let live = vec![Block::User {
        prompt_id: "new".into(),
        text: "full".into(),
        files: vec![],
    }];
    let merged = merge_live_over_disk(&disk, &live);
    assert_eq!(merged.len(), 3);
    assert!(matches!(&merged[2], Block::User { text, .. } if text == "full"));
    assert_eq!(merge_live_over_disk(&disk, &[]).len(), 3);
}

#[test]
fn blocks_to_markdown_sections() {
    let md = blocks_to_markdown(&[
        Block::User {
            prompt_id: "p".into(),
            text: "ask".into(),
            files: vec![],
        },
        Block::Assistant {
            prompt_id: "p".into(),
            text: "ans".into(),
        },
        Block::Tool {
            id: "t".into(),
            title: "read".into(),
            status: "ok".into(),
            input_preview: "f.rs".into(),
            prompt_id: "p".into(),
            result_count: None,
        },
    ]);
    assert!(md.contains("## User"));
    assert!(md.contains("ask"));
    assert!(md.contains("## Assistant"));
    assert!(md.contains("## Tools"));
    assert!(md.contains("`read`"));
}

#[test]
fn parse_updates_file_and_models_cache() {
    let dir = tempfile::tempdir().expect("tempdir");
    let updates = dir.path().join("updates.jsonl");
    fs::write(
        &updates,
        r#"{"params":{"update":{"sessionUpdate":"user_message_chunk","content":{"text":"z"}},"_meta":{"promptId":"p"}}}"#,
    )
    .expect("write jsonl");
    let parsed = parse_updates_file(&updates).expect("parse");
    assert!(matches!(&parsed.blocks[0], Block::User { text, .. } if text == "z"));

    assert!(models_from_cache(dir.path()).is_empty());
    fs::write(
        dir.path().join("models_cache.json"),
        r#"{"models":{"grok-3":{"info":{"name":"Grok 3","hidden":false}},"hid":{"info":{"hidden":true}}}}"#,
    )
    .expect("write cache");
    let models = models_from_cache(dir.path());
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "grok-3");
}
