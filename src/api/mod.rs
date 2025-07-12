use crate::error::{GrokError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub mod anthropic;
pub mod openai;
pub mod xai;

/// Message in a conversation
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Tool call made by the assistant
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool definition for API
#[derive(Serialize, Deserialize, Clone)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

/// Function definition
#[derive(Serialize, Deserialize, Clone)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}

/// Request to chat completion API
#[derive(Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: String,
    pub temperature: f64,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

/// Response format specification
#[derive(Serialize)]
pub struct ResponseFormat {
    pub r#type: String,
}

/// Response from chat completion API
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

/// Choice in API response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Choice {
    pub message: Message,
}

/// Configuration for an API client
pub struct ApiConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

/// Trait for API clients
#[async_trait]
pub trait ApiClient: Send + Sync {
    /// Get the configuration
    fn config(&self) -> &ApiConfig;

    /// Call the chat completion API
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse>;
}

/// Create an API client based on the provider
// TODO: Add support for additional providers (Cohere, Mistral, local models)
// TODO: Implement provider-specific configuration validation
// TODO: Add provider capability detection (max tokens, features supported)
pub fn create_client(provider: &str, config: ApiConfig) -> Result<Box<dyn ApiClient>> {
    match provider {
        "xai" => Ok(Box::new(xai::XaiClient::new(config))),
        "openai" => Ok(Box::new(openai::OpenAiClient::new(config))),
        "anthropic" => Ok(Box::new(anthropic::AnthropicClient::new(config))),
        _ => Err(GrokError::Config(format!(
            "Unknown API provider: {}",
            provider
        ))),
    }
}
