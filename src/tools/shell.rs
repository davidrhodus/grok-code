use super::utils::sanitize_shell_command;
use super::{Tool, ToolContext};
use serde_json::{json, Value as JsonValue};
use std::process::Command;

/// Tool for running shell commands
pub struct RunShellCommand;

impl Tool for RunShellCommand {
    fn name(&self) -> &'static str {
        "run_shell_command"
    }

    fn description(&self) -> &'static str {
        "Run a shell command and return output. Use for testing like 'cargo test'. Confirm for sensitive commands."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "The shell command to run."}
            },
            "required": ["command"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let command = match args["command"].as_str() {
            Some(cmd) => cmd.trim(),
            None => return "Error: 'command' parameter is required".to_string(),
        };

        // Sanitize the command
        let command = match sanitize_shell_command(command) {
            Ok(cmd) => cmd,
            Err(e) => return format!("Error: {e}"),
        };

        if !context.confirm_action(&format!("run command '{command}'")) {
            return "Command not confirmed.".to_string();
        }

        if context.dry_run {
            return format!("Dry-run: Would execute command: {command}");
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&context.project_root)
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);

                let mut result = String::new();

                if !o.status.success() {
                    result.push_str(&format!(
                        "Command failed with exit code: {}\n",
                        o.status.code().unwrap_or(-1)
                    ));
                }

                if !stdout.is_empty() {
                    result.push_str("Output:\n");
                    result.push_str(&stdout);
                }

                if !stderr.is_empty() {
                    if !stdout.is_empty() {
                        result.push('\n');
                    }
                    result.push_str("Error output:\n");
                    result.push_str(&stderr);
                }

                if result.is_empty() {
                    "Command executed successfully (no output)".to_string()
                } else {
                    result.trim_end().to_string()
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    "Error: 'sh' command not found. Shell might not be available.".to_string()
                }
                std::io::ErrorKind::PermissionDenied => {
                    "Error: Permission denied executing shell command".to_string()
                }
                _ => format!("Error running command: {e}"),
            },
        }
    }
}
