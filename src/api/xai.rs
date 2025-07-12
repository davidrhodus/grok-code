use super::{ApiClient, ApiConfig, ChatCompletionRequest, ChatCompletionResponse};
use crate::error::{GrokError, Result};
use async_trait::async_trait;
use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use std::time::Duration;

/// xAI API client implementation
pub struct XaiClient {
    config: ApiConfig,
    client: Client,
}

impl XaiClient {
    /// Create a new xAI client
    pub fn new(config: ApiConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ApiClient for XaiClient {
    fn config(&self) -> &ApiConfig {
        &self.config
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        // Debug logging for API requests
        if std::env::var("DEBUG_API").is_ok() {
            eprintln!("DEBUG: Sending API request to {url}");
            eprintln!("  Model: {}", request.model);
            eprintln!("  Messages count: {}", request.messages.len());
            eprintln!(
                "  Tools count: {}",
                request.tools.as_ref().map(|t| t.len()).unwrap_or(0)
            );
            eprintln!("  Tool choice: {}", request.tool_choice);
        }

        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.config.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&request)
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(GrokError::rate_limited(text, Some(60))); // Default 60 second retry
            }
            return Err(GrokError::ApiError(format!("API error {status}: {text}")));
        }

        let json_response = response.json::<ChatCompletionResponse>().await?;

        // Debug logging for xAI responses
        if std::env::var("DEBUG_API").is_ok() {
            eprintln!("DEBUG: API Response received");
            eprintln!("  Choices count: {}", json_response.choices.len());
            if !json_response.choices.is_empty() {
                let msg = &json_response.choices[0].message;
                eprintln!("  Content: {:?}", msg.content);
                eprintln!(
                    "  Tool calls: {:?}",
                    msg.tool_calls.as_ref().map(|tc| tc.len())
                );
            }
        }

        Ok(json_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Function, Message, Tool};
    use mockito::{Server, ServerGuard};
    use serde_json::json;

    async fn create_test_client(server: &ServerGuard) -> XaiClient {
        let config = ApiConfig {
            api_key: "test_key".to_string(),
            base_url: server.url(),
            model: "grok-2-latest".to_string(),
            timeout_secs: 60,
            max_retries: 3,
        };
        XaiClient::new(config)
    }

    #[tokio::test]
    async fn test_chat_completion_success() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "test-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-2-latest",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });

        let _mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer test_key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
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
            "Hello! How can I help you today?"
        );
    }

    #[tokio::test]
    async fn test_chat_completion_with_tools() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "test-456",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-2-latest",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\": \"test.txt\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 30,
                "total_tokens": 80
            }
        });

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
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

        let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "read_file");
        assert_eq!(tool_calls[0].function.arguments, "{\"path\": \"test.txt\"}");
    }

    #[tokio::test]
    async fn test_chat_completion_rate_limit() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body("Rate limit exceeded. Please try again later.")
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
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
            GrokError::RateLimitExceeded { message: msg, .. } => {
                assert!(msg.contains("Rate limit exceeded"));
            }
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_chat_completion_api_error() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(403)
            .with_body("Forbidden: Invalid API key")
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
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
            GrokError::ApiError(msg) => {
                assert!(msg.contains("403"));
                assert!(msg.contains("Invalid API key"));
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[tokio::test]
    async fn test_chat_completion_invalid_json() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("invalid json response")
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
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

        // The invalid JSON causes a reqwest deserialization error which gets wrapped in Http error
        match result.unwrap_err() {
            GrokError::Http(_) => (),
            _ => panic!("Expected Http error for invalid JSON response"),
        }
    }

    #[tokio::test]
    async fn test_chat_completion_empty_response() {
        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "test-789",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-2-latest",
            "choices": [],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 0,
                "total_tokens": 10
            }
        });

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
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
        assert_eq!(response.choices.len(), 0);
    }

    #[tokio::test]
    async fn test_debug_logging() {
        // Set DEBUG_API env var for this test
        std::env::set_var("DEBUG_API", "1");

        let mut server = Server::new_async().await;
        let client = create_test_client(&server).await;

        let mock_response = json!({
            "id": "test-debug",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-2-latest",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Debug test response"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 3,
                "total_tokens": 8
            }
        });

        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create_async()
            .await;

        let request = ChatCompletionRequest {
            model: "grok-2-latest".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("Test with debug".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: Some(vec![]),
            tool_choice: "none".to_string(),
            temperature: 1.0,
            max_tokens: 50,
            response_format: None,
        };

        // This should trigger debug logging
        let response = client.chat_completion(request).await.unwrap();
        assert_eq!(
            response.choices[0].message.content.as_ref().unwrap(),
            "Debug test response"
        );

        // Clean up
        std::env::remove_var("DEBUG_API");
    }
}
