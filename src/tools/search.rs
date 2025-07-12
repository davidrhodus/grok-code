use super::utils::sanitize_regex_pattern;
use super::{Tool, ToolContext};
use regex::Regex;
use serde_json::{json, Value as JsonValue};
use std::fs;
use walkdir::WalkDir;

/// Tool for searching the codebase
pub struct SearchCodebase;

impl Tool for SearchCodebase {
    fn name(&self) -> &'static str {
        "search_codebase"
    }

    fn description(&self) -> &'static str {
        "Search for text in the codebase files, optionally using regex."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "The search query or regex pattern."},
                "is_regex": {"type": "boolean", "description": "Whether to treat query as regex.", "default": false}
            },
            "required": ["query"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let query = match args["query"].as_str() {
            Some(q) => q,
            None => return "Error: 'query' parameter is required".to_string(),
        };

        if query.is_empty() {
            return "Error: Query cannot be empty".to_string();
        }

        let is_regex = args["is_regex"].as_bool().unwrap_or(false);
        let mut results = String::new();

        if std::env::var("DEBUG_API").is_ok() {
            use colored::*;
            eprintln!(
                "{}: Searching for '{}' (regex: {})",
                "DEBUG".blue().bold(),
                query.cyan(),
                is_regex.to_string().yellow()
            );
        }

        if is_regex {
            let validated_pattern = match sanitize_regex_pattern(query) {
                Ok(p) => p,
                Err(e) => return format!("Error: {e}"),
            };

            let re = match Regex::new(&validated_pattern) {
                Ok(re) => re,
                Err(e) => return format!("Invalid regex: {e}"),
            };

            for e in WalkDir::new(&context.project_root).into_iter().flatten() {
                if e.file_type().is_file() {
                    if let Ok(content) = fs::read_to_string(e.path()) {
                        if re.is_match(&content) {
                            results.push_str(&format!("Match in {}\n", e.path().display()));
                        }
                    }
                }
            }
        } else {
            for e in WalkDir::new(&context.project_root).into_iter().flatten() {
                if e.file_type().is_file() {
                    if let Ok(content) = fs::read_to_string(e.path()) {
                        if content.contains(query) {
                            results.push_str(&format!(
                                "Found '{}' in {}\n",
                                query,
                                e.path().display()
                            ));
                        }
                    }
                }
            }
        }

        if results.is_empty() {
            "No matches found.".to_string()
        } else {
            if std::env::var("DEBUG_API").is_ok() {
                use colored::*;
                let match_count = results.lines().count();
                eprintln!(
                    "{}: Found {} matches",
                    "DEBUG".blue().bold(),
                    match_count.to_string().green()
                );
            }
            results
        }
    }
}
