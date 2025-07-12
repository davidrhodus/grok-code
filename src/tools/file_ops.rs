use super::utils::sanitize_path;
use super::{Tool, ToolContext};
use crate::backup::BackupManager;
use ropey::Rope;
use serde_json::{json, Value as JsonValue};
use std::fs::{self, File};

/// Tool for reading file contents
pub struct ReadFile;

impl Tool for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file at the given path."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "The path to the file."}
            },
            "required": ["path"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let path_str = match args["path"].as_str() {
            Some(p) => p,
            None => return "Error: 'path' parameter is required".to_string(),
        };

        let path = match sanitize_path(path_str, &context.project_root) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };
        match fs::read_to_string(&path) {
            Ok(content) => {
                if std::env::var("DEBUG_API").is_ok() {
                    use colored::*;
                    eprintln!(
                        "{}: Read file {} ({} bytes, {} lines)",
                        "DEBUG".blue().bold(),
                        path.display().to_string().cyan(),
                        content.len().to_string().yellow(),
                        content.lines().count().to_string().yellow()
                    );
                }
                content
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    format!("Error: File not found: {}", path.display())
                }
                std::io::ErrorKind::PermissionDenied => {
                    format!("Error: Permission denied reading file: {}", path.display())
                }
                _ => format!("Error reading file: {e}"),
            },
        }
    }
}

/// Tool for writing file contents
pub struct WriteFile;

impl Tool for WriteFile {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file at the given path. Overwrites if exists. Creates timestamped backup with retention policy."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "The path to the file."},
                "content": {"type": "string", "description": "The content to write."}
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let path_str = match args["path"].as_str() {
            Some(p) => p,
            None => return "Error: 'path' parameter is required".to_string(),
        };

        let content = match args["content"].as_str() {
            Some(c) => c,
            None => return "Error: 'content' parameter is required".to_string(),
        };

        let path = match sanitize_path(path_str, &context.project_root) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };

        // Create backup if file exists
        let backup_manager = BackupManager::new(None);
        let backup_result = if path.exists() {
            match backup_manager.create_backup(&path) {
                Ok(backup_path) => Some(backup_path),
                Err(e) => return format!("Error creating backup: {e}"),
            }
        } else {
            None
        };

        if !context.confirm_action(&format!("write to {}", path.display())) {
            return "Write operation not confirmed.".to_string();
        }

        if context.dry_run {
            match backup_result {
                Some(backup_path) => format!(
                    "Dry-run: Would write to {} (backed up to {}):\n{}",
                    path.display(),
                    backup_path.display(),
                    content
                ),
                None => format!("Dry-run: Would write to {}:\n{}", path.display(), content),
            }
        } else {
            match fs::write(&path, content) {
                Ok(_) => {
                    if std::env::var("DEBUG_API").is_ok() {
                        use colored::*;
                        eprintln!(
                            "{}: Wrote file {} ({} bytes)",
                            "DEBUG".blue().bold(),
                            path.display().to_string().cyan(),
                            content.len().to_string().yellow()
                        );
                    }
                    match backup_result {
                        Some(backup_path) => format!(
                            "File written successfully (backed up to {}).",
                            backup_path.display()
                        ),
                        None => "File written successfully.".to_string(),
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::PermissionDenied => format!(
                        "Error: Permission denied writing to file: {}",
                        path.display()
                    ),
                    _ => format!("Error writing file: {e}"),
                },
            }
        }
    }
}

/// Tool for editing specific lines in a file
pub struct EditFile;

impl Tool for EditFile {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Edit specific lines in a file. Creates timestamped backup with retention policy."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "The path to the file."},
                "start_line": {"type": "number", "description": "Starting line number (1-indexed)."},
                "end_line": {"type": "number", "description": "Ending line number (inclusive)."},
                "new_content": {"type": "string", "description": "The new content for the specified lines."}
            },
            "required": ["path", "start_line", "end_line", "new_content"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let path_str = match args["path"].as_str() {
            Some(p) => p,
            None => return "Error: 'path' parameter is required".to_string(),
        };

        let start_line = match args["start_line"].as_u64() {
            Some(n) => n as usize,
            None => {
                return "Error: 'start_line' parameter is required and must be a number".to_string()
            }
        };

        let end_line = match args["end_line"].as_u64() {
            Some(n) => n as usize,
            None => {
                return "Error: 'end_line' parameter is required and must be a number".to_string()
            }
        };

        let new_content = match args["new_content"].as_str() {
            Some(c) => c,
            None => return "Error: 'new_content' parameter is required".to_string(),
        };

        if start_line == 0 || end_line < start_line {
            return "Error: Invalid line numbers. Lines are 1-indexed and end_line must be >= start_line.".to_string();
        }

        let path = match sanitize_path(path_str, &context.project_root) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };

        let backup_manager = BackupManager::new(None);
        let backup_path = if path.exists() {
            match backup_manager.create_backup(&path) {
                Ok(backup_path) => backup_path,
                Err(e) => return format!("Error creating backup: {e}"),
            }
        } else {
            return format!("Error: File not found: {}", path.display());
        };

        if !context.confirm_action(&format!("edit {}", path.display())) {
            return "Edit operation not confirmed.".to_string();
        }

        if context.dry_run {
            return format!(
                "Dry-run: Would edit {} lines {}-{} with:\n{} (backed up to {})",
                path.display(),
                start_line,
                end_line,
                new_content,
                backup_path.display()
            );
        }

        match fs::read_to_string(&path) {
            Ok(file_content) => {
                let mut rope = Rope::from_str(&file_content);

                // Check if line numbers are valid
                let total_lines = rope.len_lines();
                if start_line > total_lines {
                    return format!(
                        "Error: start_line {start_line} exceeds total lines in file ({total_lines})"
                    );
                }

                let start_char = if start_line > 1 {
                    rope.line_to_char(start_line - 1)
                } else {
                    0
                };
                let end_char = rope.line_to_char(end_line.min(total_lines));
                rope.remove(start_char..end_char);
                rope.insert(start_char, new_content);

                match File::create(&path) {
                    Ok(mut file) => match rope.write_to(&mut file) {
                        Ok(_) => format!(
                            "File edited successfully (backed up to {}).",
                            backup_path.display()
                        ),
                        Err(e) => format!("Error writing to file: {e}"),
                    },
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            format!("Error: Permission denied creating file: {}", path.display())
                        }
                        _ => format!("Error creating file: {e}"),
                    },
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    format!("Error: File not found: {}", path.display())
                }
                std::io::ErrorKind::PermissionDenied => {
                    format!("Error: Permission denied reading file: {}", path.display())
                }
                _ => format!("Error reading file: {e}"),
            },
        }
    }
}

/// Tool for listing directory contents
pub struct ListFiles;

impl Tool for ListFiles {
    fn name(&self) -> &'static str {
        "list_files"
    }

    fn description(&self) -> &'static str {
        "List contents of a directory."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "The directory path (defaults to '.')."}
            },
            "required": []
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let path_str = args["path"].as_str().unwrap_or(".");
        let path = match sanitize_path(path_str, &context.project_root) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };

        match fs::read_dir(&path) {
            Ok(entries) => {
                let mut files = Vec::new();
                for entry in entries.flatten() {
                    files.push(entry.file_name().to_string_lossy().to_string());
                }
                files.sort(); // Sort for consistent output
                files.join("\n")
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    format!("Error: Directory not found: {}", path.display())
                }
                std::io::ErrorKind::PermissionDenied => format!(
                    "Error: Permission denied reading directory: {}",
                    path.display()
                ),
                _ => format!("Error listing directory: {e}"),
            },
        }
    }
}
