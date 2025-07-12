use async_trait::async_trait;
use grok_code::api::{
    ApiClient, ApiConfig, ChatCompletionRequest, ChatCompletionResponse, Choice, Message,
};
use grok_code::error::{GrokError, Result};

/// Mock API client for testing
struct MockApiClient {
    config: ApiConfig,
    responses: Vec<ChatCompletionResponse>,
}

impl MockApiClient {
    fn new(config: ApiConfig, responses: Vec<ChatCompletionResponse>) -> Self {
        Self { config, responses }
    }
}

#[async_trait]
impl ApiClient for MockApiClient {
    fn config(&self) -> &ApiConfig {
        &self.config
    }

    async fn chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        // For testing, we'll just return the first response
        if self.responses.is_empty() {
            return Err(GrokError::ApiError("No mock responses".to_string()));
        }
        // Since we can't modify self, we'll just return the first response
        Ok(ChatCompletionResponse {
            choices: self.responses[0].choices.clone(),
        })
    }
}

#[tokio::test]
async fn test_mock_api_simple_response() {
    let config = ApiConfig {
        base_url: "http://mock.api".to_string(),
        api_key: "mock_key".to_string(),
        model: "mock-model".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let mock_response = ChatCompletionResponse {
        choices: vec![Choice {
            message: Message {
                role: "assistant".to_string(),
                content: Some("Hello! I'm a mock response.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        }],
    };

    let client = MockApiClient::new(config, vec![mock_response]);

    let request = ChatCompletionRequest {
        model: "mock-model".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: None,
        tool_choice: "none".to_string(),
        temperature: 0.7,
        max_tokens: 100,
        response_format: None,
    };

    let response = client.chat_completion(request).await.unwrap();
    assert_eq!(response.choices.len(), 1);
    assert_eq!(
        response.choices[0].message.content.as_ref().unwrap(),
        "Hello! I'm a mock response."
    );
}

#[tokio::test]
async fn test_mock_api_tool_calls() {
    use grok_code::api::{FunctionCall, ToolCall};

    let config = ApiConfig {
        base_url: "http://mock.api".to_string(),
        api_key: "mock_key".to_string(),
        model: "mock-model".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let mock_response = ChatCompletionResponse {
        choices: vec![Choice {
            message: Message {
                role: "assistant".to_string(),
                content: None,
                tool_calls: Some(vec![ToolCall {
                    id: "call_123".to_string(),
                    r#type: "function".to_string(),
                    function: FunctionCall {
                        name: "read_file".to_string(),
                        arguments: r#"{"path": "test.txt"}"#.to_string(),
                    },
                }]),
                tool_call_id: None,
            },
        }],
    };

    let client = MockApiClient::new(config, vec![mock_response]);

    let request = ChatCompletionRequest {
        model: "mock-model".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some("Read test.txt".to_string()),
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
    assert!(response.choices[0].message.tool_calls.is_some());
    let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].function.name, "read_file");
}

#[tokio::test]
async fn test_mock_api_error_handling() {
    let config = ApiConfig {
        base_url: "http://mock.api".to_string(),
        api_key: "mock_key".to_string(),
        model: "mock-model".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    // Empty responses vector will cause error on first call
    let client = MockApiClient::new(config, vec![]);

    let request = ChatCompletionRequest {
        model: "mock-model".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: None,
        tool_choice: "none".to_string(),
        temperature: 0.7,
        max_tokens: 100,
        response_format: None,
    };

    let result = client.chat_completion(request).await;
    assert!(result.is_err());
    match result {
        Err(GrokError::ApiError(msg)) => assert_eq!(msg, "No mock responses"),
        _ => panic!("Expected ApiError"),
    }
}
