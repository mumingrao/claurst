pub mod provider_id;
pub use provider_id::{ProviderId, ModelId};

pub mod session_storage;
pub use sqlite_storage::{SqliteSessionStore, SessionSummary};

pub mod attachments;

pub mod git_utils;

pub mod auth_store;
pub use auth_store::{AuthStore, StoredCredential};

pub mod device_code;

pub mod token_budget;
pub mod truncate;
pub mod format_utils;
pub mod crypto_utils;
pub mod status_notices;
pub mod auto_mode;

pub mod remote_session;
pub mod cloud_session;

pub mod claudemd;

pub mod message_utils;

pub mod file_history;

pub mod snapshot;

pub mod goal;
pub use goal::{Goal, GoalError, GoalStatus, GoalStore, MAX_GOAL_TURNS, MAX_OBJECTIVE_CHARS,
               goal_continuation_message, goal_kickoff_message, goal_system_prompt_addendum, goals_enabled};

pub mod feature_flags;

pub mod mcp_templates;

pub mod ide;
pub use ide::{IdeKind, detect_ide};

pub mod update_check;
pub use update_check::{check_for_updates, UpdateInfo};

pub use error::{ClaudeError, Result};
pub use types::{
    ContentBlock, ImageSource, DocumentSource, CitationsConfig, Message, MessageContent,
    MessageCost, Role, ToolDefinition, ToolResultContent, UsageInfo,
};
pub use config::{AgentDefinition, BudgetSplitPolicy, Config, CommandTemplate, FormatterConfig, ManagedAgentConfig, ManagedAgentPreset, McpServerConfig, OutputFormat, PermissionMode, ProviderConfig, Settings, SkillsConfig, Theme, builtin_managed_agent_presets, default_agents, strip_jsonc_comments, substitute_env_vars};
pub use import_config::{ClaudeMdPreview, ImportExecutionResult, ImportPaths, ImportPreview, ImportSelection, PreviewAction, PreviewField, SettingsPreview, build_import_preview, execute_import, summarize_import_result};

pub mod skill_discovery;
pub use skill_discovery::{DiscoveredSkill, discover_skills, parse_skill_file};
pub use cost::CostTracker;
pub use history::ConversationSession;
pub use feature_flags::FeatureFlagManager;
pub use permissions::{
    AutoPermissionHandler, InteractivePermissionHandler,
    ManagedAutoPermissionHandler, ManagedInteractivePermissionHandler,
    PermissionAction, PermissionDecision, PermissionHandler,
    PermissionLevel, PermissionManager, PermissionRequest,
    PermissionRule, PermissionScope, SerializedPermissionRule,
    format_permission_reason,
};

pub mod error {
    use thiserror::Error;

    /// The unified error type for Claurst.
    #[derive(Error, Debug)]
    pub enum ClaudeError {
        #[error("API error: {0}")]
        Api(String),

        #[error("API error {status}: {message}")]
        ApiStatus { status: u16, message: String },

        #[error("Authentication error: {0}")]
        Auth(String),

        #[error("Permission denied: {0}")]
        PermissionDenied(String),

        #[error("Tool error: {0}")]
        Tool(String),

        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),

        #[error("JSON error: {0}")]
        Json(#[from] serde_json::Error),

        #[error("HTTP error: {0}")]
        Http(#[from] reqwest::Error),

        #[error("Rate limit exceeded")]
        RateLimit,

        #[error("Context window exceeded")]
        ContextWindowExceeded,

        #[error("Max tokens reached")]
        MaxTokensReached,

        #[error("Cancelled")]
        Cancelled,

        #[error("Configuration error: {0}")]
        Config(String),

        #[error("MCP error: {0}")]
        Mcp(String),

        #[error("{0}")]
        Other(String),
    }

    pub type Result<T> = std::result::Result<T, ClaudeError>;

    impl ClaudeError {
        pub fn is_retryable(&self) -> bool {
            matches!(
                self,
                ClaudeError::RateLimit
                    | ClaudeError::ApiStatus { status: 429, .. }
                    | ClaudeError::ApiStatus { status: 529, .. }
            )
        }

        pub fn is_context_limit(&self) -> bool {
            matches!(
                self,
                ClaudeError::ContextWindowExceeded | ClaudeError::MaxTokensReached
            )
        }
    }
}

pub mod types {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "lowercase")]
    pub enum Role {
        User,
        Assistant,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum ContentBlock {
        Text {
            text: String,
        },
        Image {
            source: ImageSource,
        },
        ToolUse {
            id: String,
            name: String,
            input: Value,
        },
        ToolResult {
            tool_use_id: String,
            content: ToolResultContent,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_error: Option<bool>,
        },
        Thinking {
            thinking: String,
            signature: String,
        },
        RedactedThinking {
            data: String,
        },
        Document {
            source: DocumentSource,
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            context: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            citations: Option<CitationsConfig>,
        },
        UserLocalCommandOutput {
            command: String,
            output: String,
        },
        UserCommand {
            name: String,
            args: String,
        },
        UserMemoryInput {
            key: String,
            value: String,
        },
        SystemAPIError {
            message: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            retry_secs: Option<u32>,
        },
        CollapsedReadSearch {
            tool_name: String,
            paths: Vec<String>,
            n_hidden: usize,
        },
        TaskAssignment {
            id: String,
            subject: String,
            description: String,
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum ToolResultContent {
        Text(String),
        Blocks(Vec<ContentBlock>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ImageSource {
        #[serde(rename = "type")]
        pub source_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DocumentSource {
        #[serde(rename = "type")]
        pub source_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CitationsConfig {
        pub enabled: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Message {
        pub role: Role,
        pub content: MessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub uuid: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cost: Option<MessageCost>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub snapshot_patch: Option<crate::snapshot::Patch>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum MessageContent {
        Text(String),
        Blocks(Vec<ContentBlock>),
    }

    impl Message {
        pub fn user(content: impl Into<String>) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Text(content.into()),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn user_blocks(blocks: Vec<ContentBlock>) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(blocks),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn assistant(content: impl Into<String>) -> Self {
            Self {
                role: Role::Assistant,
                content: MessageContent::Text(content.into()),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
            Self {
                role: Role::Assistant,
                content: MessageContent::Blocks(blocks),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn get_text(&self) -> Option<&str> {
            match &self.content {
                MessageContent::Text(t) => Some(t.as_str()),
                MessageContent::Blocks(blocks) => blocks.iter().find_map(|b| {
                    if let ContentBlock::Text{ text} = b {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
            }
        }

        pub fn get_all_text(&self) -> String {
            match &self.content {
                MessageContent::Text(t) => t.clone(),
                MessageContent::Blocks(blocks) => blocks
                    .iter()
                    .filter_map(|b| {
                        if let ContentBlock::Text { text } = b {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            }
        }

        pub fn gt_tool_use_blocks(&self) -> Vec<&ContentBlock> {
            match &self.content {
                MessageContent::Blocks(blocks) => blocks
                    .iter()
                    .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
                    .collect(),
                _ => vec![]
            }
        }

        pub fn get_tool_result_blocks(&self) -> Vec<&ContentBlock> {
            match &self.content {
                MessageContent::Blocks(blocks) => blocks
                    .iter()
                    .filter(|b| matches!(b, ContentBlock::ToolResult { .. }))
                    .collect(),
                _ => vec![],
            }
        }

        pub fn content_blocks(&self) -> Vec<ContentBlock> {
            match &self.content {
                MessageContent::Text(t) => vec![ContentBlock::Text { text: t.clone() }],
                MessageContent::Blocks(b) => b.clone()
            }
        }

        pub fn has_tool_use(&self) -> bool {
            !self.get_tool_result_blocks().is_empty()
        }

        pub fn user_local_command_output(command: impl Into<String>, output: impl Into<String>) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::UserLocalCommandOutput { 
                    command: command.into(), 
                    output: output.into(),
                }]),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn user_command(name: impl Into<String>, args: impl Into<String>) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::UserCommand { 
                    name: name.into(), 
                    args: args.into() 
                }]),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn user_memory_input(key: impl Into<String>, value: impl Into<String>) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::UserMemoryInput { 
                    key: key.into(), 
                    value: value.into() 
                }]),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn system_api_error(message: impl Into<String>, retry_secs: Option<u32>) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::SystemAPIError {
                    message: message.into(),
                    retry_secs,
                }]),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn collapsed_read_search(
            tool_name: impl Into<String>,
            paths: Vec<String>,
            n_hidden: usize,
        ) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::CollapsedReadSearch {
                    tool_name: tool_name.into(),
                    paths,
                    n_hidden,
                }]),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }

        pub fn task_assignment(
            id: impl Into<String>,
            subject: impl Into<String>,
            description: impl Into<String>,
        ) -> Self {
            Self {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::TaskAssignment {
                    id: id.into(),
                    subject: subject.into(),
                    description: description.into(),
                }]),
                uuid: None,
                cost: None,
                snapshot_patch: None,
            }
        }
    }
}