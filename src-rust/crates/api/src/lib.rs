use claurst_core::constants::{ANTHROPIC_API_VERSION, ANTHROPIC_BETA_HEADER};
use claurst_core::error::ClaudeError;
use claurst_core::types::{ContentBlock, Message, MessageContent, Role, ToolDefinition, UsageInfo};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

pub mod cch;
pub mod codex_adapter;

pub mod provider_types;
pub mod provider_error;

pub mod provider;
pub mod auth;
pub mod stream_parser;
pub mod transform;

pub mod registry;

pub mod providers;

pub mod model_registry;

pub mod error_handling;

pub mod transformers;

pub use client::AnthropicClient;
pub use streaming::{AnthropicStreamEvent, StreamHandler};
pub use types::*;

pub use provider_types::*;
pub use provider_error::ProviderError;

pub use provider::{LlmProvider, ModelInfo};
pub use auth::{AuthProvider, LoginFlow};
pub use stream_parser::{StreamParser, SseStreamParser, JsonLinesStreamParser};
pub use transform::MessageTransformer;

pub use registry::ProviderRegistry;

pub use providers::AnthropicProvider;
pub use providers::GoogleProvider;
pub use providers::MinimaxProvider;
pub use providers::OpenAiProvider;

pub use model_registry::{
    CostBreakdown, ExperimentalMode, InterleavedReasoning, Modality, ModelEntry, ModelRegistry,
    ModelStatus, ProviderEntry, ProviderOverride, effective_model_for_config,
};

pub use error_handling::{is_context_overflow, parse_error_response, RetryConfig};

pub use providers::AzureProvider;
pub use providers::BedrockProvider;
pub use providers::CopilotProvider;

pub use providers::{
    OpenAiCompatProvider,
    ollama, lm_studio, deepseek, groq, xai, openrouter, mistral,
};

pub use providers::CohereProvider;

pub use transformers::{AnthropicTransformer, OpenAiChatTransformer};

pub mod types {
    use super::*;

    #[derive(Debug, Clone, Serialize)]
    pub struct CreateMessageRequest {
        pub model: String,
        pub max_tokens: u32,
        pub messages: Vec<ApiMessage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub system: Option<SystemPrompt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tools: Option<Vec<ApiToolDefinition>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub temperature: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub top_p: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub top_k: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub stop_sequences: Option<Vec<String>>,
        pub stream: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub thinking: Option<ThinkingConfig>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ThinkingConfig {
        #[serde(rename = "type")]
        pub thinking_type: String,
        pub buget_tokens: u32,
    }

    impl ThinkingConfig {
        pub fn enabled(buget: u32) -> Self {
            Self {
                thinking_type: "enabled".to_string(),
                buget_tokens: buget,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum SystemPrompt {
        Text(String),
        Blocks(Vec<SystemBlock>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SystemBlock {
        #[serde(rename = "type")]
        pub block_type: String,
        pub text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_control: Option<CacheControl>,
    }

     #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CacheControl {
        #[serde(rename = "type")]
        pub control_type: String,
    }

    impl CacheControl {
        pub fn ephemeral() -> Self {
            Self {
                control_type: "ephemeral".to_string(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ApiMessage {
        pub role: String,
        pub content: Value,
    }

    impl From<&Message> for ApiMessage {
        fn from(msg: &Message) -> Self {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            let content = match &msg.content {
                MessageContent::Text(t) => Value::String(t.clone()),
                MessageContent::Blocks(blocks) => {
                    serde_json::to_value(blocks).unwrap_or(Value::Null)
                }
            };
            Self {
                role: role.to_string(),
                content,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ApiToolDefinition {
        pub name: String,
        pub description: String,
        pub input_schema: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_control: Option<CacheControl>,
    }

    impl From<&ToolDefinition> for ApiToolDefinition {
        fn from(td: &ToolDefinition) -> Self {
            Self {
                name: td.name.clone(),
                description: td.description.clone(),
                input_schema: td.input_schema.clone(),
                cache_control: None,
            }
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CreateMessageResponse {
        pub id: String,
        #[serde(rename = "type")]
        pub response_type: String,
        pub role: String,
        pub content: Vec<Value>,
        pub model: String,
        pub stop_reason: Option<String>,
        pub stop_sequence: Option<String>,
        pub usage: UsageInfo,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ApiErrorResponse {
        #[serde(rename = "type")]
        pub error_type: String,
        pub error: ApiErrorDetail,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct ApiErrorDetail {
        #[serde(rename = "type")]
        pub error_type: String,
        pub message: String,
    }
}