use super::{Tool, ToolContext};
use serde_json::{json, Value as JsonValue};

/// Tool for debugging code based on error messages
pub struct DebugCode;

impl Tool for DebugCode {
    fn name(&self) -> &'static str {
        "debug_code"
    }

    fn description(&self) -> &'static str {
        "Analyze error messages and suggest fixes. Searches codebase for keywords."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "error_message": {"type": "string", "description": "The error message to analyze."}
            },
            "required": ["error_message"]
        })
    }

    fn execute(&self, args: &JsonValue, _context: &ToolContext<'_>) -> String {
        let error_message = args["error_message"].as_str().unwrap_or("");
        let keywords: Vec<&str> = error_message
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        let mut results = String::new();

        // For each keyword, search the codebase
        for keyword in keywords {
            let _search_args = json!({
                "query": keyword,
                "is_regex": false
            });

            // TODO: Implement a way to call other tools from within a tool
            // TODO: Add smarter error pattern recognition (stack traces, common errors)
            // TODO: Integrate with language-specific error databases
            results.push_str(&format!("Search for '{keyword}' in codebase\n"));
        }

        format!("Debug analysis for error: {error_message}\n{results}\nUse this to suggest fixes.")
    }
}

/// Tool for analyzing log files
pub struct AnalyzeLog;

impl Tool for AnalyzeLog {
    fn name(&self) -> &'static str {
        "analyze_log"
    }

    fn description(&self) -> &'static str {
        "Analyze log files for patterns, errors, and potential issues."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "log_content": {"type": "string", "description": "The log content to analyze."},
                "max_lines": {"type": "number", "description": "Maximum lines to analyze (default: 1000).", "default": 1000}
            },
            "required": ["log_content"]
        })
    }

    fn execute(&self, args: &JsonValue, _context: &ToolContext<'_>) -> String {
        let log_content = args["log_content"].as_str().unwrap_or("");
        let max_lines = args["max_lines"].as_u64().unwrap_or(1000) as usize;

        // Analyze log patterns
        let lines: Vec<&str> = log_content.lines().take(max_lines).collect();
        let total_lines = lines.len();

        // Count different log levels
        let mut error_count = 0;
        let mut warn_count = 0;
        let mut info_count = 0;
        let mut debug_count = 0;

        // Track error messages and stack traces
        let mut error_messages: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut stack_traces: Vec<String> = Vec::new();
        let mut in_stack_trace = false;
        let mut current_stack_trace = String::new();

        // Common error patterns
        let error_patterns = [
            ("panic", "Panic/crash detected"),
            ("segfault", "Segmentation fault"),
            ("null pointer", "Null pointer access"),
            ("out of memory", "Memory exhaustion"),
            ("timeout", "Operation timeout"),
            ("connection refused", "Network connection issue"),
            ("permission denied", "Permission/access issue"),
            ("file not found", "Missing file/resource"),
            ("assertion failed", "Assertion failure"),
        ];

        let mut detected_patterns: Vec<&str> = Vec::new();

        for line in &lines {
            let line_lower = line.to_lowercase();

            // Count log levels
            if line_lower.contains("error")
                || line_lower.contains("err ")
                || line_lower.contains("err:")
            {
                error_count += 1;

                // Extract error message
                if let Some(msg_start) = line.find("ERROR:").or_else(|| line.find("Error:")) {
                    let msg = line[msg_start..].trim();
                    *error_messages.entry(msg.to_string()).or_insert(0) += 1;
                }
            } else if line_lower.contains("warn") || line_lower.contains("warning") {
                warn_count += 1;
            } else if line_lower.contains("info") {
                info_count += 1;
            } else if line_lower.contains("debug") || line_lower.contains("trace") {
                debug_count += 1;
            }

            // Detect stack traces
            if line.trim_start().starts_with("at ")
                || line.contains("backtrace:")
                || line.contains("stack trace:")
            {
                if !in_stack_trace {
                    in_stack_trace = true;
                    current_stack_trace.clear();
                }
                current_stack_trace.push_str(line);
                current_stack_trace.push('\n');
            } else if in_stack_trace && line.trim().is_empty() {
                if !current_stack_trace.is_empty() {
                    stack_traces.push(current_stack_trace.clone());
                    current_stack_trace.clear();
                }
                in_stack_trace = false;
            }

            // Check for known error patterns
            for (pattern, description) in &error_patterns {
                if line_lower.contains(pattern) && !detected_patterns.contains(description) {
                    detected_patterns.push(description);
                }
            }
        }

        // Save any remaining stack trace
        if in_stack_trace && !current_stack_trace.is_empty() {
            stack_traces.push(current_stack_trace);
        }

        // Build analysis report
        let mut report = String::new();
        report.push_str("🔍 Log Analysis Report\n");
        report.push_str("======================\n\n");

        report.push_str(&format!("📊 Summary (analyzed {total_lines} lines):\n"));
        report.push_str(&format!(
            "   • Errors: {} ({:.1}%)\n",
            error_count,
            (error_count as f64 / total_lines as f64) * 100.0
        ));
        report.push_str(&format!(
            "   • Warnings: {} ({:.1}%)\n",
            warn_count,
            (warn_count as f64 / total_lines as f64) * 100.0
        ));
        report.push_str(&format!(
            "   • Info: {} ({:.1}%)\n",
            info_count,
            (info_count as f64 / total_lines as f64) * 100.0
        ));
        report.push_str(&format!(
            "   • Debug: {} ({:.1}%)\n\n",
            debug_count,
            (debug_count as f64 / total_lines as f64) * 100.0
        ));

        if !detected_patterns.is_empty() {
            report.push_str("⚠️  Detected Issues:\n");
            for pattern in &detected_patterns {
                report.push_str(&format!("   • {pattern}\n"));
            }
            report.push('\n');
        }

        if !error_messages.is_empty() {
            report.push_str("❌ Top Error Messages:\n");
            let mut errors: Vec<_> = error_messages.iter().collect();
            errors.sort_by(|a, b| b.1.cmp(a.1));
            for (msg, count) in errors.iter().take(5) {
                report.push_str(&format!("   • {msg} ({count}x)\n"));
            }
            report.push('\n');
        }

        if !stack_traces.is_empty() {
            report.push_str(&format!("📋 Found {} stack trace(s)\n", stack_traces.len()));
            if let Some(trace) = stack_traces.first() {
                report.push_str("   First stack trace:\n");
                for line in trace.lines().take(5) {
                    report.push_str(&format!("   {line}\n"));
                }
                if trace.lines().count() > 5 {
                    report.push_str("   ...\n");
                }
            }
            report.push('\n');
        }

        // Provide recommendations
        report.push_str("💡 Recommendations:\n");

        if error_count > total_lines / 10 {
            report.push_str("   • High error rate detected - investigate root cause\n");
        }

        if detected_patterns.contains(&"Memory exhaustion") {
            report
                .push_str("   • Memory issues detected - check for leaks or increase heap size\n");
        }

        if detected_patterns.contains(&"Network connection issue") {
            report.push_str(
                "   • Network issues detected - verify connectivity and firewall settings\n",
            );
        }

        if detected_patterns.contains(&"Permission/access issue") {
            report.push_str("   • Permission issues detected - check file/directory permissions\n");
        }

        if stack_traces.len() > 5 {
            report.push_str(
                "   • Multiple crashes detected - application stability is compromised\n",
            );
        }

        if warn_count > error_count * 2 {
            report.push_str(
                "   • Many warnings present - consider addressing to prevent future errors\n",
            );
        }

        report
    }
}

/// Tool for running linter
pub struct RunLint;

impl Tool for RunLint {
    fn name(&self) -> &'static str {
        "run_lint"
    }

    fn description(&self) -> &'static str {
        "Run cargo clippy to check for code issues. Pass fix=true to attempt automatic fixes."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "fix": {"type": "boolean", "description": "Whether to attempt automatic fixes.", "default": false}
            },
            "required": []
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let fix = args["fix"].as_bool().unwrap_or(false);
        let cmd = if fix {
            "cargo clippy --fix"
        } else {
            "cargo clippy"
        };

        // Execute through shell command
        let shell_args = json!({
            "command": cmd
        });

        // We'll use the shell command tool via the context
        use crate::tools::shell::RunShellCommand;
        let shell_tool = RunShellCommand;
        shell_tool.execute(&shell_args, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_context() -> ToolContext<'static> {
        ToolContext {
            project_root: PathBuf::from("/tmp"),
            dry_run: false,
            no_confirm: true,
            git_repo: None,
            tui_mode: false,
        }
    }

    #[test]
    fn test_analyze_log_basic() {
        let tool = AnalyzeLog;
        let context = create_test_context();

        let log_content = r#"
[2024-01-01 10:00:00] INFO: Application started
[2024-01-01 10:00:01] DEBUG: Loading configuration
[2024-01-01 10:00:02] WARN: Config file not found, using defaults
[2024-01-01 10:00:03] ERROR: Database connection failed
[2024-01-01 10:00:04] ERROR: Database connection failed
[2024-01-01 10:00:05] INFO: Retrying connection
[2024-01-01 10:00:06] INFO: Connection established
"#;

        let args = json!({
            "log_content": log_content
        });

        let result = tool.execute(&args, &context);

        // Check that the report contains expected elements
        assert!(result.contains("Log Analysis Report"));
        assert!(result.contains("Summary (analyzed"));
        assert!(result.contains("Errors: 2"));
        assert!(result.contains("Warnings: 1"));
        assert!(result.contains("Info: 3"));
        assert!(result.contains("Debug: 1"));
        // Check for error messages section
        assert!(result.contains("Top Error Messages:"));
        assert!(result.contains("ERROR: Database connection failed (2x)"));
    }

    #[test]
    fn test_analyze_log_with_stack_trace() {
        let tool = AnalyzeLog;
        let context = create_test_context();

        let log_content = r#"
[2024-01-01 10:00:00] ERROR: Null pointer exception
stack trace:
  at MyClass.method1(MyClass.java:42)
  at MyClass.method2(MyClass.java:87)
  at Main.main(Main.java:15)

[2024-01-01 10:00:01] INFO: Attempting recovery
"#;

        let args = json!({
            "log_content": log_content
        });

        let result = tool.execute(&args, &context);

        assert!(result.contains("Found 1 stack trace(s)"));
        assert!(result.contains("at MyClass.method1"));
        assert!(result.contains("Null pointer access"));
    }

    #[test]
    fn test_analyze_log_error_patterns() {
        let tool = AnalyzeLog;
        let context = create_test_context();

        let log_content = r#"
[2024-01-01 10:00:00] ERROR: Connection refused to server
[2024-01-01 10:00:01] ERROR: Out of memory: Java heap space
[2024-01-01 10:00:02] ERROR: Permission denied: /var/log/app.log
[2024-01-01 10:00:03] ERROR: Timeout waiting for response
[2024-01-01 10:00:04] PANIC: Segmentation fault
"#;

        let args = json!({
            "log_content": log_content
        });

        let result = tool.execute(&args, &context);

        // Check detected patterns
        assert!(result.contains("Detected Issues:"));
        assert!(result.contains("Network connection issue"));
        assert!(result.contains("Memory exhaustion"));
        assert!(result.contains("Permission/access issue"));
        assert!(result.contains("Operation timeout"));
        assert!(result.contains("Panic/crash detected"));

        // Check recommendations
        assert!(result.contains("Recommendations:"));
        assert!(result.contains("Memory issues detected"));
        assert!(result.contains("Network issues detected"));
        assert!(result.contains("Permission issues detected"));
    }

    #[test]
    fn test_analyze_log_high_error_rate() {
        let tool = AnalyzeLog;
        let context = create_test_context();

        // Create log with > 10% error rate
        let mut log_content = String::new();
        for i in 0..100 {
            if i < 15 {
                log_content.push_str(&format!("[2024-01-01 10:00:{i:02}] ERROR: Error {i}\n"));
            } else {
                log_content.push_str(&format!(
                    "[2024-01-01 10:00:{i:02}] INFO: Normal operation\n"
                ));
            }
        }

        let args = json!({
            "log_content": log_content
        });

        let result = tool.execute(&args, &context);

        assert!(result.contains("High error rate detected"));
    }

    #[test]
    fn test_analyze_log_max_lines() {
        let tool = AnalyzeLog;
        let context = create_test_context();

        // Create a large log
        let mut log_content = String::new();
        for i in 0..50 {
            log_content.push_str(&format!("[2024-01-01 10:00:{i:02}] INFO: Line {i}\n"));
        }

        let args = json!({
            "log_content": log_content,
            "max_lines": 10
        });

        let result = tool.execute(&args, &context);

        // Should only analyze 10 lines
        assert!(result.contains("analyzed 10 lines"));
    }
}
