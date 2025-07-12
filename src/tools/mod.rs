//! # Tools Module
//!
//! This module contains all the tools available to the AI agent. Tools are organized
//! into categories based on their functionality.
//!
//! ## Tool Categories
//!
//! - **File Operations** ([`file_ops`]): Read, write, edit files
//! - **Shell Commands** ([`shell`]): Execute system commands
//! - **Search** ([`search`]): Search through codebases
//! - **Analysis** ([`analysis`]): Code analysis, debugging, linting
//! - **Git Operations** ([`git_ops`]): Git commits, PRs, merge conflict resolution
//! - **External Services** ([`external`]): Web search, Jira integration
//!
//! ## Creating Custom Tools
//!
//! To create a custom tool, implement the [`Tool`] trait:
//!
//! ```
//! use grok_code::tools::{Tool, ToolContext};
//! use serde_json::{json, Value as JsonValue};
//!
//! struct MyTool;
//!
//! impl Tool for MyTool {
//!     fn name(&self) -> &'static str {
//!         "my_tool"
//!     }
//!     
//!     fn description(&self) -> &'static str {
//!         "Description of what my tool does"
//!     }
//!     
//!     fn parameters(&self) -> JsonValue {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "param": {"type": "string", "description": "A parameter"}
//!             },
//!             "required": ["param"]
//!         })
//!     }
//!     
//!     fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
//!         // Tool implementation
//!         format!("Executed with: {}", args["param"].as_str().unwrap_or(""))
//!     }
//! }
//! ```

use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::Mutex;

// Global mutex for stdin access during confirmations
static STDIN_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

// Re-export all tool modules
/// Tools for code analysis, debugging, and linting
pub mod analysis;
/// Backup management tools
pub mod backup_ops;
/// Tools for external service integration (web search, Jira)
pub mod external;
/// Tools for file system operations
pub mod file_ops;
/// Tools for git operations (commits, PRs, merge conflicts)
pub mod git_ops;
/// Tools for searching through code
pub mod search;
/// Tools for executing shell commands
pub mod shell;
/// Utility functions for tools
pub mod utils;

// Re-export commonly used items
pub use analysis::{AnalyzeLog, DebugCode, RunLint};
pub use backup_ops::{CleanBackups, ListBackups};
pub use external::{CreateJiraTicket, WebSearch};
pub use file_ops::{EditFile, ListFiles, ReadFile, WriteFile};
pub use git_ops::{CreateCommit, ResolveMergeConflict, SubmitPR};
pub use search::SearchCodebase;
pub use shell::RunShellCommand;

/// Trait that all tools must implement
pub trait Tool {
    /// Get the name of the tool
    fn name(&self) -> &'static str;

    /// Get the description of the tool
    fn description(&self) -> &'static str;

    /// Get the parameters schema for the tool
    fn parameters(&self) -> JsonValue;

    /// Execute the tool with the given arguments
    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String;
}

/// Context provided to tools during execution
pub struct ToolContext<'a> {
    pub project_root: PathBuf,
    pub dry_run: bool,
    pub no_confirm: bool,
    pub git_repo: Option<&'a git2::Repository>,
}

impl ToolContext<'_> {
    /// Resolve a path relative to the project root
    pub fn resolve_path(&self, path_str: &str) -> PathBuf {
        use std::path::Path;

        if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            self.project_root.join(path_str)
        }
    }

    /// Confirm an action with the user
    pub fn confirm_action(&self, action: &str) -> bool {
        use std::io::{self, IsTerminal, Write};

        if self.no_confirm || self.dry_run {
            return true;
        }

        // Check if stdin is piped/non-interactive
        if !io::stdin().is_terminal() {
            println!(); // New line for clarity
            println!("⚠️  Non-interactive mode: auto-confirming {action} ");
            return true;
        }

        // Lock stdin access to ensure only one confirmation prompt at a time
        let _lock = STDIN_MUTEX.lock().unwrap();

        println!(); // New line for clarity
        print!("❓ Confirm {action}? (y/n): ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => input.trim().to_lowercase() == "y",
            Err(_) => false,
        }
    }
}

/// Registry for all available tools
// TODO: Implement dynamic tool loading from plugins
// TODO: Add tool categories and filtering
// TODO: Implement tool dependency resolution
// TODO: Add tool usage analytics and metrics
#[derive(Clone)]
pub struct ToolRegistry {
    tools: std::sync::Arc<Vec<Box<dyn Tool + Send + Sync>>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new tool registry with all available tools
    pub fn new() -> Self {
        let mut tools: Vec<Box<dyn Tool + Send + Sync>> = vec![
            Box::new(ReadFile),
            Box::new(WriteFile),
            Box::new(EditFile),
            Box::new(ListFiles),
            Box::new(RunShellCommand),
            Box::new(SearchCodebase),
            Box::new(DebugCode),
            Box::new(AnalyzeLog),
            Box::new(RunLint),
            Box::new(ResolveMergeConflict),
            Box::new(CreateCommit),
            Box::new(SubmitPR),
            Box::new(WebSearch),
            Box::new(CreateJiraTicket),
            Box::new(ListBackups),
            Box::new(CleanBackups),
        ];

        // Load plugins if enabled
        if std::env::var("GROK_PLUGINS").unwrap_or_else(|_| "true".to_string()) == "true" {
            match Self::load_plugins() {
                Ok(plugin_tools) => {
                    let count = plugin_tools.len();
                    if count > 0 {
                        println!("✅ Loaded {count} plugin tools");
                    }
                    tools.extend(plugin_tools);
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to load plugins: {e}");
                }
            }
        }

        Self {
            tools: std::sync::Arc::new(tools),
        }
    }

    /// Load plugins from configured directories
    fn load_plugins() -> Result<Vec<Box<dyn Tool + Send + Sync>>, Box<dyn std::error::Error>> {
        use crate::plugins::{default_plugin_directories, PluginLoader};

        let mut loader = PluginLoader::new();
        let mut _loaded_any = false;

        // Check custom plugin directory from environment
        if let Ok(custom_dir) = std::env::var("GROK_PLUGIN_DIR") {
            let path = std::path::PathBuf::from(custom_dir);
            if path.exists() {
                loader.load_from_directory(&path)?;
                _loaded_any = true;
            }
        }

        // Check default directories
        for dir in default_plugin_directories() {
            if dir.exists() {
                match loader.load_from_directory(&dir) {
                    Ok(_) => _loaded_any = true,
                    Err(e) => eprintln!(
                        "Warning: Failed to load plugins from {}: {}",
                        dir.display(),
                        e
                    ),
                }
            }
        }

        // Check for single plugin file
        if let Ok(plugin_file) = std::env::var("GROK_PLUGIN_FILE") {
            let path = std::path::PathBuf::from(plugin_file);
            if path.exists() {
                loader.load_from_file(&path)?;
                _loaded_any = true;
            }
        }

        Ok(loader.create_tools())
    }

    /// Get all tools as a vector
    pub fn get_tools(&self) -> &[Box<dyn Tool + Send + Sync>] {
        &self.tools
    }

    /// Find a tool by name
    pub fn find_tool(&self, name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools
            .iter()
            .find(|tool| tool.name() == name)
            .map(|tool| tool.as_ref())
    }

    /// Execute a tool by name with the given arguments
    pub fn execute_tool(&self, name: &str, args_str: &str, context: &ToolContext<'_>) -> String {
        use std::time::Instant;

        let args: JsonValue = match serde_json::from_str(args_str) {
            Ok(v) => v,
            Err(e) => return format!("Invalid arguments: {e}"),
        };

        match self.find_tool(name) {
            Some(tool) => {
                let start = Instant::now();
                let result = tool.execute(&args, context);

                if std::env::var("DEBUG_API").is_ok() {
                    use colored::*;
                    let elapsed = start.elapsed();
                    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
                    let elapsed_str = if elapsed_ms < 100.0 {
                        format!("{elapsed_ms:.2}ms").green()
                    } else if elapsed_ms < 1000.0 {
                        format!("{elapsed_ms:.2}ms").yellow()
                    } else {
                        format!("{elapsed_ms:.2}ms").red()
                    };
                    eprintln!(
                        "{}: Tool '{}' executed in {}",
                        "DEBUG".blue().bold(),
                        name.cyan(),
                        elapsed_str
                    );
                    eprintln!("  {} {}", "Args:".dimmed(), args_str);
                    eprintln!(
                        "  {} {} chars",
                        "Result length:".dimmed(),
                        result.len().to_string().yellow()
                    );
                }

                result
            }
            None => "Unknown tool.".to_string(),
        }
    }
}
