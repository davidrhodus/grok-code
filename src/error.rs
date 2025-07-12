use thiserror::Error;

/// Custom error type for grok-code
#[derive(Error, Debug)]
pub enum GrokError {
    /// IO errors (file operations, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// API errors
    #[error("API error: {0}")]
    Api(String),

    /// API request errors
    #[error("API request error: {0}")]
    ApiRequestError(String),

    /// API response errors
    #[error("{0}")]
    ApiError(String),

    /// Rate limit exceeded with retry information
    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { 
        message: String,
        retry_after: Option<u64>,
    },

    /// No summary generated
    #[error("No summary generated")]
    NoSummaryGenerated,

    /// JSON parsing errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(String),

    /// HTTP request errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Git operation errors
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// Regex errors
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Tool execution errors
    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Process execution error
    #[error("Process execution error: {0}")]
    ProcessExecution(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Error with additional context
    #[error("{context}")]
    WithContext {
        context: String,
        #[source]
        source: Box<GrokError>,
    },
}

impl GrokError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an API error
    pub fn api(msg: impl Into<String>) -> Self {
        Self::Api(msg.into())
    }

    /// Create a tool execution error
    pub fn tool_execution(msg: impl Into<String>) -> Self {
        Self::ToolExecution(msg.into())
    }

    /// Create a rate limit error with optional retry information
    pub fn rate_limited(msg: impl Into<String>, retry_after: Option<u64>) -> Self {
        Self::RateLimitExceeded {
            message: msg.into(),
            retry_after,
        }
    }

    /// Add context to an existing error
    pub fn context(self, context: impl Into<String>) -> Self {
        match self {
            // Don't double-wrap context errors
            Self::WithContext { .. } => self,
            _ => Self::WithContext {
                context: context.into(),
                source: Box::new(self),
            },
        }
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded { .. } | Self::Timeout(_) | Self::Http(_)
        )
    }

    /// Get retry delay in seconds if applicable
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Self::RateLimitExceeded { retry_after, .. } => *retry_after,
            Self::Timeout(_) => Some(5), // Default 5 second retry for timeouts
            Self::Http(_) => Some(2),     // Default 2 second retry for HTTP errors
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, GrokError>;

// Helper trait for adding context to Results
pub trait ResultExt<T> {
    fn context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.context(context))
    }
}

// TODO: Add structured error reporting for better user experience
// TODO: Implement error recovery strategies based on error types
