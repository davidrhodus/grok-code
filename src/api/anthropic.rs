use super::{ApiClient, ApiConfig, ChatCompletionRequest, ChatCompletionResponse, Choice, Message};
use crate::error::{GrokError, Result};
use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;

/// Anthropic API client implementation
pub struct AnthropicClient {
    config: ApiConfig,
    client: Client,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub fn new(config: ApiConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Convert OpenAI-style request to Anthropic format
    fn convert_request(&self, request: ChatCompletionRequest) -> AnthropicRequest {
        let mut messages = Vec::new();
        let mut system_prompt = None;

        // Extract system message and convert other messages
        for msg in request.messages {
            match msg.role.as_str() {
                "system" => {
                    system_prompt = msg.content;
                }
                "user" => {
                    messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content.unwrap_or_default(),
                    });
                }
                "assistant" => {
                    messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: msg.content.unwrap_or_default(),
                    });
                }
                _ => {} // Skip other roles
            }
        }

        // Convert tools if present
        let tools = request.tools.map(|tools| {
            tools
                .into_iter()
                .map(|tool| AnthropicTool {
                    name: tool.function.name,
                    description: tool.function.description,
                    input_schema: tool.function.parameters,
                })
                .collect()
        });

        AnthropicRequest {
            model: self.config.model.clone(),
            messages,
            system: system_prompt,
            max_tokens: request.max_tokens,
            temperature: Some(request.temperature),
            tools,
        }
    }

    /// Convert Anthropic response to OpenAI format
    fn convert_response(&self, response: AnthropicResponse) -> ChatCompletionResponse {
        let tool_calls = if !response.content.is_empty() {
            let tool_uses: Vec<_> = response
                .content
                .iter()
                .filter_map(|c| {
                    if c.r#type == "tool_use" {
                        Some(super::ToolCall {
                            id: c.id.clone().unwrap_or_default(),
                            r#type: "function".to_string(),
                            function: super::FunctionCall {
                                name: c.name.clone().unwrap_or_default(),
                                arguments: serde_json::to_string(&c.input).unwrap_or_default(),
                            },
                        })
                    } else {
                        None
                    }
                })
                .collect();

            if tool_uses.is_empty() {
                None
            } else {
                Some(tool_uses)
            }
        } else {
            None
        };

        let text_content = response
            .content
            .iter()
            .filter_map(|c| {
                if c.r#type == "text" {
                    c.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let content = if text_content.is_empty() {
            None
        } else {
            Some(text_content)
        };

        ChatCompletionResponse {
            choices: vec![Choice {
                message: Message {
                    role: "assistant".to_string(),
                    content,
                    tool_calls,
                    tool_call_id: None,
                },
            }],
        }
    }
}

#[async_trait]
impl ApiClient for AnthropicClient {
    fn config(&self) -> &ApiConfig {
        &self.config
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let url = format!("{}/messages", self.config.base_url);

        // Debug logging for API requests
        if std::env::var("DEBUG_API").is_ok() {
            eprintln!("DEBUG: Sending Anthropic API request to {url}");
            eprintln!("  Model: {}", request.model);
            eprintln!("  Messages count: {}", request.messages.len());
            eprintln!(
                "  Tools count: {}",
                request.tools.as_ref().map(|t| t.len()).unwrap_or(0)
            );
        }

        // Convert request to Anthropic format
        let anthropic_request = self.convert_request(request);

        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|_| GrokError::Config("Invalid API key format".to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        if !response.status().is_success() {
            let _status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Rate limit exceeded".to_string());
            return Err(GrokError::rate_limited(text, Some(60))); // Default 60 second retry
        }

        let anthropic_response = response.json::<AnthropicResponse>().await?;

        // Debug logging for Anthropic responses
        if std::env::var("DEBUG_API").is_ok() {
            eprintln!("DEBUG: Anthropic API Response received");
            eprintln!("  Content blocks: {}", anthropic_response.content.len());
            eprintln!("  Stop reason: {:?}", anthropic_response.stop_reason);
        }

        // Convert response to OpenAI format
        Ok(self.convert_response(anthropic_response))
    }
}

// Anthropic-specific request/response types
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: JsonValue,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Function, Tool};
    use mockito::{Server, ServerGuard};
    use serde_json::json;

    async fn create_test_client(server: &ServerGuard) -> AnthropicClient {
        let config = ApiConfig {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            model: "claude-3-opus-20240229".to_string(),
            timeout_secs: 60,
            max_retries: 3,
        };
        AnthropicClient::new(config)
    }

    #[tokio::test]
    async fn test_chat_completion_success() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello from Claude!"
            }],
            "model": "claude-3-opus-20240229",
            "stop_reason": "end_turn"
        });

        let _mock = server
            .mock("POST", "/messages")
            .match_header("x-api-key", "test_key")
            .match_header("content-type", "application/json")
            .match_header("anthropic-version", "2023-06-01")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            tool_choice: "auto".to_string(),
            temperature: 0.7,
            max_tokens: 100,
            response_format: None,
        };

        let response = client.chat_completion(request).await.unwrap();

        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content.as_ref().unwrap(),
            "Hello from Claude!"
        );
    }

    #[tokio::test]
    async fn test_chat_completion_with_tools() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "I'll read that file for you."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "read_file",
                    "input": {"path": "test.txt"}
                }
            ],
            "model": "claude-3-opus-20240229",
            "stop_reason": "tool_use"
        });

        let _mock = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("Read the test.txt file".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: Some(vec![Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "read_file".to_string(),
                    description: "Read a file".to_string(),
                    parameters: json!({"type": "object"}),
                },
            }]),
            tool_choice: "auto".to_string(),
            temperature: 0.7,
            max_tokens: 100,
            response_format: None,
        };

        let response = client.chat_completion(request).await.unwrap();

        assert_eq!(
            response.choices[0].message.content.as_ref().unwrap(),
            "I'll read that file for you."
        );
        let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "read_file");
        assert!(tool_calls[0].function.arguments.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_system_message_handling() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "msg_789",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "I understand the system context."
            }],
            "model": "claude-3-opus-20240229",
            "stop_reason": "end_turn"
        });

        let _mock = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: Some("You are a helpful assistant.".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            tools: None,
            tool_choice: "auto".to_string(),
            temperature: 0.7,
            max_tokens: 100,
            response_format: None,
        };

        let response = client.chat_completion(request).await.unwrap();
        assert_eq!(
            response.choices[0].message.content.as_ref().unwrap(),
            "I understand the system context."
        );
    }

    #[tokio::test]
    async fn test_rate_limit_error() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let error_response = json!({
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded"
            }
        });

        let _mock = server
            .mock("POST", "/messages")
            .with_status(429)
            .with_body(error_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            tool_choice: "auto".to_string(),
            temperature: 0.7,
            max_tokens: 100,
            response_format: None,
        };

        let result = client.chat_completion(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            GrokError::RateLimitExceeded { .. } => (),
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }
}
