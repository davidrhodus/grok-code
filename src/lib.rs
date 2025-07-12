//! # Grok Code - AI Coding Assistant
//!
//! Grok Code is a powerful CLI tool that provides a Claude-like coding assistant experience
//! using multiple AI providers: xAI's Grok, OpenAI's GPT, or Anthropic's Claude.
//!
//! ## Features
//!
//! - **Multi-Provider Support**: Switch between xAI Grok, OpenAI, and Anthropic Claude
//! - **Comprehensive Tool Set**: File operations, shell commands, code search, git integration
//! - **Response Caching**: Intelligent caching for improved performance
//! - **Secure Key Storage**: System keychain integration for API keys
//! - **Safety Features**: Dry-run mode, automatic backups, confirmation prompts
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`agent`]: The main AI agent that orchestrates interactions
//! - [`api`]: API client implementations for different providers
//! - [`tools`]: Collection of tools the agent can use
//! - [`cache`]: Response caching system
//! - [`error`]: Error types and handling
//! - [`keystore`]: Secure API key management
//!
//! ## Example Usage
//!
//! ```no_run
//! use grok_code::agent::GrokAgent;
//! use grok_code::api::ApiConfig;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ApiConfig {
//!     api_key: "your-api-key".to_string(),
//!     base_url: "https://api.x.ai/v1".to_string(),
//!     model: "grok-2".to_string(),
//!     timeout_secs: 300,
//!     max_retries: 3,
//! };
//!
//! let mut agent = GrokAgent::new(
//!     "xai",
//!     config,
//!     PathBuf::from("."),
//!     false,  // dry_run
//!     3,      // max_depth
//!     false,  // no_confirm
//! )?;
//!
//! agent.process_prompt("Analyze my code", true).await;
//! # Ok(())
//! # }
//! ```

/// The AI agent module containing the main orchestration logic
pub mod agent;

/// API client implementations for different AI providers
pub mod api;

/// Response caching system for improved performance
pub mod cache;

/// Error types and error handling utilities
pub mod error;

/// Secure keystore for API key management
pub mod keystore;

/// Collection of tools available to the AI agent
pub mod tools;

/// Plugin system for loading custom tools
pub mod plugins;

/// Terminal user interface module
pub mod tui;

/// Backup management with retention
pub mod backup;
