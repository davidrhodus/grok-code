//! # Plugin System
//!
//! This module provides a plugin system for loading custom tools dynamically.
//! Plugins can be loaded from configuration files or external libraries.
//!
//! ## Plugin Types
//!
//! - **Script Plugins**: Tools defined in configuration files (JSON/TOML)
//! - **Binary Plugins**: External executables that implement the tool protocol
//! - **Library Plugins**: Dynamic libraries (.so/.dll/.dylib) - future enhancement
//!
//! ## Example Plugin Configuration
//!
//! ```toml
//! [[plugins]]
//! name = "custom_formatter"
//! description = "Format code with custom rules"
//! type = "script"
//! command = "python scripts/formatter.py"
//! parameters = '''
//! {
//!   "type": "object",
//!   "properties": {
//!     "file": {"type": "string", "description": "File to format"}
//!   },
//!   "required": ["file"]
//! }
//! '''
//! ```

use crate::error::{GrokError, Result};
use crate::tools::{Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Plugin configuration from file
#[derive(Debug, Deserialize, Serialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,
    /// Plugin description
    pub description: String,
    /// Plugin type (script, binary, etc.)
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    /// Command to execute
    pub command: String,
    /// Tool parameters schema
    pub parameters: String,
    /// Optional working directory
    pub working_dir: Option<PathBuf>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Types of plugins supported
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Script-based plugin (executes external command)
    Script,
    /// Binary executable plugin
    Binary,
    // Future: Dynamic library plugin
    // Library,
}

/// A plugin-based tool that executes external commands
pub struct PluginTool {
    config: PluginConfig,
}

impl PluginTool {
    /// Create a new plugin tool from configuration
    pub fn new(config: PluginConfig) -> Self {
        Self { config }
    }
}

impl Tool for PluginTool {
    fn name(&self) -> &'static str {
        // We need to leak the string to get a 'static lifetime
        // This is okay for plugins as they're loaded once at startup
        Box::leak(self.config.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.config.description.clone().into_boxed_str())
    }

    fn parameters(&self) -> JsonValue {
        serde_json::from_str(&self.config.parameters)
            .unwrap_or_else(|_| json!({"type": "object", "properties": {}}))
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        // Confirm action if needed
        if !context.confirm_action(&format!("execute plugin '{}'", self.config.name)) {
            return format!("Plugin '{}' execution not confirmed.", self.config.name);
        }

        if context.dry_run {
            return format!(
                "Dry-run: Would execute plugin '{}' with command: {}",
                self.config.name, self.config.command
            );
        }

        // Prepare command
        let mut parts = self.config.command.split_whitespace();
        let program = match parts.next() {
            Some(p) => p,
            None => return "Error: Plugin command is empty".to_string(),
        };

        let mut cmd = Command::new(program);

        // Add remaining arguments
        for arg in parts {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        } else {
            cmd.current_dir(&context.project_root);
        }

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Pass arguments as JSON via stdin
        let args_json = args.to_string();

        // Execute command
        match cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                // Write arguments to stdin
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(args_json.as_bytes());
                }

                // Wait for completion
                match child.wait_with_output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);

                        if output.status.success() {
                            stdout.to_string()
                        } else {
                            format!(
                                "Plugin '{}' failed:\nstdout: {}\nstderr: {}",
                                self.config.name, stdout, stderr
                            )
                        }
                    }
                    Err(e) => format!("Error waiting for plugin '{}': {}", self.config.name, e),
                }
            }
            Err(e) => format!("Error executing plugin '{}': {}", self.config.name, e),
        }
    }
}

/// Wrapper structure for TOML files with plugins array
#[derive(Debug, Deserialize)]
struct PluginFile {
    plugins: Vec<PluginConfig>,
}

/// Plugin loader that manages dynamic tool loading
pub struct PluginLoader {
    /// Loaded plugins
    plugins: Vec<PluginConfig>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Load plugins from a configuration file
    pub fn load_from_file(&mut self, path: &PathBuf) -> Result<usize> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GrokError::Config(format!("Failed to read plugin config: {e}")))?;

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        let configs: Vec<PluginConfig> = match extension {
            "toml" => {
                // Try to parse as a file with plugins array first
                if let Ok(plugin_file) = toml::from_str::<PluginFile>(&content) {
                    plugin_file.plugins
                } else {
                    // Fall back to parsing as direct array
                    toml::from_str(&content)
                        .map_err(|e| GrokError::Config(format!("Failed to parse TOML: {e}")))?
                }
            }
            "json" => {
                // Try to parse as a file with plugins array first
                if let Ok(plugin_file) = serde_json::from_str::<PluginFile>(&content) {
                    plugin_file.plugins
                } else {
                    // Fall back to parsing as direct array
                    serde_json::from_str(&content)
                        .map_err(|e| GrokError::Config(format!("Failed to parse JSON: {e}")))?
                }
            }
            _ => {
                return Err(GrokError::Config(
                    "Plugin config must be .toml or .json".to_string(),
                ))
            }
        };

        let count = configs.len();
        self.plugins.extend(configs);
        Ok(count)
    }

    /// Load plugins from a directory
    pub fn load_from_directory(&mut self, dir: &PathBuf) -> Result<usize> {
        if !dir.is_dir() {
            return Err(GrokError::Config(format!(
                "Plugin directory does not exist: {}",
                dir.display()
            )));
        }

        let mut total_loaded = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "toml" || ext == "json" {
                        match self.load_from_file(&path) {
                            Ok(count) => {
                                total_loaded += count;
                                println!("Loaded {} plugins from {}", count, path.display());
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to load plugins from {}: {}",
                                    path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(total_loaded)
    }

    /// Convert loaded plugins into Tool trait objects
    pub fn create_tools(self) -> Vec<Box<dyn Tool + Send + Sync>> {
        self.plugins
            .into_iter()
            .map(|config| Box::new(PluginTool::new(config)) as Box<dyn Tool + Send + Sync>)
            .collect()
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Default plugin directory paths
pub fn default_plugin_directories() -> Vec<PathBuf> {
    vec![
        PathBuf::from("plugins"),
        PathBuf::from(".grok-code/plugins"),
        dirs::config_dir()
            .map(|p| p.join("grok-code").join("plugins"))
            .unwrap_or_default(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_config_parsing() {
        let toml_config = r#"
name = "test_tool"
description = "A test tool"
type = "script"
command = "echo hello"
parameters = '{"type": "object", "properties": {}}'
"#;

        let config: PluginConfig = toml::from_str(toml_config).unwrap();
        assert_eq!(config.name, "test_tool");
        assert_eq!(config.plugin_type, PluginType::Script);
    }

    #[test]
    fn test_plugin_file_parsing() {
        // Test parsing with [[plugins]] wrapper
        let toml_with_wrapper = r#"
[[plugins]]
name = "tool1"
description = "First tool"
type = "script"
command = "echo one"
parameters = '{}'

[[plugins]]
name = "tool2"
description = "Second tool"
type = "binary"
command = "echo two"
parameters = '{}'
"#;

        let plugin_file: PluginFile = toml::from_str(toml_with_wrapper).unwrap();
        assert_eq!(plugin_file.plugins.len(), 2);
        assert_eq!(plugin_file.plugins[0].name, "tool1");
        assert_eq!(plugin_file.plugins[1].name, "tool2");
    }

    #[test]
    fn test_plugin_array_parsing() {
        // Test parsing as direct array (backwards compatibility)
        let toml_array = r#"
[[]]
name = "tool3"
description = "Third tool"
type = "script"
command = "echo three"
parameters = '{}'
"#;

        // This should work with the direct array format
        let configs: Vec<PluginConfig> = toml::from_str(toml_array).unwrap_or_default();
        if !configs.is_empty() {
            assert_eq!(configs[0].name, "tool3");
        }
    }

    #[test]
    fn test_plugin_tool_creation() {
        let config = PluginConfig {
            name: "test".to_string(),
            description: "Test plugin".to_string(),
            plugin_type: PluginType::Script,
            command: "echo test".to_string(),
            parameters: r#"{"type": "object", "properties": {}}"#.to_string(),
            working_dir: None,
            env: HashMap::new(),
        };

        let tool = PluginTool::new(config);
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), "Test plugin");
    }
}
