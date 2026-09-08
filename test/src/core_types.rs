use ggok_core::types::{Block, SessionMeta};
use std::path::PathBuf;

#[test]
fn block_prompt_id() {
    let b = Block::Assistant {
        prompt_id: "abc".into(),
        text: "t".into(),
    };
    assert_eq!(b.prompt_id(), "abc");
    let end = Block::TurnEnd {
        prompt_id: "abc".into(),
        duration_ms: 0,
        cancelled: false,
    };
    assert_eq!(end.prompt_id(), "abc");
    let sub = Block::Subagent {
        prompt_id: "p".into(),
        id: "s".into(),
        child_session_id: "s".into(),
        description: "d".into(),
        subagent_type: "general-purpose".into(),
        status: "running".into(),
        duration_ms: 0,
        error: None,
    };
    assert_eq!(sub.prompt_id(), "p");
}

#[test]
fn session_meta_to_row_is_disk_idle() {
    let row = SessionMeta {
        id: "id".into(),
        cwd: "/tmp".into(),
        title: "t".into(),
        created_at: "c".into(),
        updated_at: "u".into(),
        updated_sort: 1,
        model: "m".into(),
        effort: "xhigh".into(),
        agent_name: "a".into(),
        num_messages: 3,
        parent_id: None,
        subagent_of: None,
        last_turn_summary: None,
        empty: false,
        dir: PathBuf::from("/tmp"),
    }
    .to_row();
    assert_eq!(row.source, "disk");
    assert!(!row.running);
    assert!(!row.pinned);
    assert_eq!(row.num_messages, 3);
}
