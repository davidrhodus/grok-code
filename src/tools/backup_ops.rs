//! Backup management tools

use super::utils::sanitize_path;
use super::{Tool, ToolContext};
use crate::backup::BackupManager;
use serde_json::{json, Value as JsonValue};

/// Tool for listing backups
pub struct ListBackups;

impl Tool for ListBackups {
    fn name(&self) -> &'static str {
        "list_backups"
    }

    fn description(&self) -> &'static str {
        "List all backups for a given file"
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the original file"}
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

        let backup_manager = BackupManager::new(None);
        match backup_manager.list_backups(&path) {
            Ok(backups) => {
                if backups.is_empty() {
                    format!("No backups found for {}", path.display())
                } else {
                    let mut output = format!("Backups for {}:\n", path.display());
                    for backup in backups {
                        output.push_str(&format!(
                            "  {} - {} bytes - {}\n",
                            backup.path.display(),
                            backup.size,
                            backup.created.format("%Y-%m-%d %H:%M:%S")
                        ));
                    }
                    output
                }
            }
            Err(e) => format!("Error listing backups: {e}"),
        }
    }
}

/// Tool for cleaning old backups
pub struct CleanBackups;

impl Tool for CleanBackups {
    fn name(&self) -> &'static str {
        "clean_backups"
    }

    fn description(&self) -> &'static str {
        "Clean up old backups based on retention policy"
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the original file (clean its backups)"},
                "all": {"type": "boolean", "description": "Clean all old backups in the project", "default": false}
            }
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let clean_all = args["all"].as_bool().unwrap_or(false);

        if clean_all {
            if !context.confirm_action("clean all old backups in the project") {
                return "Cleanup not confirmed.".to_string();
            }

            if context.dry_run {
                return "Dry-run: Would clean all old backups in the project".to_string();
            }

            // Clean backups for all files in the project
            let mut total_removed = 0;
            let backup_manager = BackupManager::new(None);

            for entry in walkdir::WalkDir::new(&context.project_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if let Ok(removed) = backup_manager.cleanup_old_backups(entry.path()) {
                    total_removed += removed.len();
                }
            }

            format!("Cleaned up {total_removed} old backup(s) from the project")
        } else {
            let path_str = match args["path"].as_str() {
                Some(p) => p,
                None => {
                    return "Error: Either 'path' parameter or 'all' flag is required".to_string()
                }
            };

            let path = match sanitize_path(path_str, &context.project_root) {
                Ok(p) => p,
                Err(e) => return format!("Error: {e}"),
            };

            if !context.confirm_action(&format!("clean old backups for {}", path.display())) {
                return "Cleanup not confirmed.".to_string();
            }

            if context.dry_run {
                return format!("Dry-run: Would clean old backups for {}", path.display());
            }

            let backup_manager = BackupManager::new(None);
            match backup_manager.cleanup_old_backups(&path) {
                Ok(removed) => {
                    if removed.is_empty() {
                        format!("No old backups to clean for {}", path.display())
                    } else {
                        format!(
                            "Cleaned up {} old backup(s) for {}",
                            removed.len(),
                            path.display()
                        )
                    }
                }
                Err(e) => format!("Error cleaning backups: {e}"),
            }
        }
    }
}
