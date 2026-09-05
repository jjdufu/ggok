use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRow {
    pub cwd: String,
    pub sessions: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRow {
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: String,
    pub agent_name: String,
    pub num_messages: u64,
    pub parent_id: Option<String>,
    pub empty: bool,
    pub running: bool,
    pub source: String,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub updated_sort: i64,
    pub model: String,
    pub agent_name: String,
    pub num_messages: u64,
    pub parent_id: Option<String>,
    pub empty: bool,
    pub dir: PathBuf,
}

impl SessionMeta {
    #[must_use]
    pub fn to_row(&self) -> SessionRow {
        SessionRow {
            id: self.id.clone(),
            cwd: self.cwd.clone(),
            title: self.title.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            model: self.model.clone(),
            agent_name: self.agent_name.clone(),
            num_messages: self.num_messages,
            parent_id: self.parent_id.clone(),
            empty: self.empty,
            running: false,
            source: "disk".to_string(),
            pinned: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    User {
        prompt_id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<PromptFile>,
    },
    Thought {
        prompt_id: String,
        text: String,
    },
    Assistant {
        prompt_id: String,
        text: String,
    },
    Tool {
        id: String,
        title: String,
        status: String,
        input_preview: String,
        #[serde(default)]
        prompt_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result_count: Option<u64>,
    },
    TurnEnd {
        prompt_id: String,
        #[serde(default)]
        duration_ms: u64,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        cancelled: bool,
    },
}

impl Block {
    #[must_use]
    pub fn prompt_id(&self) -> &str {
        match self {
            Self::User { prompt_id, .. }
            | Self::Thought { prompt_id, .. }
            | Self::Assistant { prompt_id, .. }
            | Self::Tool { prompt_id, .. }
            | Self::TurnEnd { prompt_id, .. } => prompt_id,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDetail {
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub source: String,
    pub writable: bool,
    #[serde(default)]
    pub running: bool,
    pub blocks: Vec<Block>,
    pub usage: TokenUsage,
    pub context: ContextUse,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_started_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub efforts: Vec<EffortInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortInfo {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueItem {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<PromptFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptFile {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextUse {
    pub used: u64,
    pub window: u64,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub recorded: bool,
    pub input_tokens: u64,
    pub cached_tokens: u64,
    pub cache_creation_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub model_calls: u64,
    pub api_duration_ms: u64,
    pub cost_usd_ticks: u64,
    pub num_turns: u64,
    pub models: Vec<ModelUsageRow>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ModelUsageRow {
    pub model: String,
    pub input_tokens: u64,
    pub cached_tokens: u64,
    pub cache_creation_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub model_calls: u64,
    pub api_duration_ms: u64,
    pub cost_usd_ticks: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDetail {
    pub content: serde_json::Value,
    pub raw_output: serde_json::Value,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub log: String,
}

#[derive(Debug, Deserialize)]
pub struct SummaryFile {
    pub info: SummaryInfo,
    #[serde(default)]
    pub generated_title: Option<String>,
    #[serde(default)]
    pub session_summary: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub current_model_id: Option<String>,
    #[serde(default)]
    pub agent_name: Option<String>,
    #[serde(default)]
    pub num_messages: u64,
    #[serde(default)]
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SummaryInfo {
    pub id: String,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubagentMeta {
    #[serde(default)]
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub child_session_id: Option<String>,
}
