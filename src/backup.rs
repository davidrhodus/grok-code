//! Backup management with retention policies
//! 
//! This module provides functionality for creating timestamped backups
//! and cleaning up old backups based on retention policies.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local, Utc};
use crate::error::{GrokError, Result};

/// Default retention period in days if not specified
const DEFAULT_RETENTION_DAYS: u64 = 7;

/// Backup manager for handling file backups with retention
pub struct BackupManager {
    /// Number of days to retain backups (0 = keep forever)
    retention_days: u64,
}

impl BackupManager {
    /// Create a new backup manager with the specified retention period
    pub fn new(retention_days: Option<u64>) -> Self {
        let retention_days = retention_days
            .or_else(|| {
                std::env::var("GROK_BACKUP_RETENTION_DAYS")
                    .ok()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(DEFAULT_RETENTION_DAYS);

        Self { retention_days }
    }

    /// Create a timestamped backup of a file
    pub fn create_backup(&self, file_path: &Path) -> Result<PathBuf> {
        if !file_path.exists() {
            return Err(GrokError::FileNotFound(
                format!("Cannot backup non-existent file: {}", file_path.display())
            ));
        }

        // Generate timestamped backup filename
        let backup_path = self.generate_backup_path(file_path);

        // Create backup
        fs::copy(file_path, &backup_path)
            .map_err(GrokError::Io)?;

        // Clean up old backups if retention is enabled
        if self.retention_days > 0 {
            self.cleanup_old_backups(file_path)?;
        }

        Ok(backup_path)
    }

    /// Generate a timestamped backup path for a file
    fn generate_backup_path(&self, file_path: &Path) -> PathBuf {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        let backup_name = format!("{}.{}.bak", file_name, timestamp);
        
        // Place backup in same directory as original file
        let mut backup_path = file_path.to_path_buf();
        backup_path.set_file_name(backup_name);
        backup_path
    }

    /// Clean up backups older than retention period
    pub fn cleanup_old_backups(&self, original_file: &Path) -> Result<Vec<PathBuf>> {
        if self.retention_days == 0 {
            return Ok(Vec::new()); // No cleanup if retention is disabled
        }

        let mut removed = Vec::new();
        let parent_dir = original_file.parent()
            .ok_or_else(|| GrokError::InvalidInput("File has no parent directory".to_string()))?;

        let file_name = original_file.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GrokError::InvalidInput("Invalid file name".to_string()))?;

        // Get current time
        let now = SystemTime::now();
        let retention_duration = std::time::Duration::from_secs(self.retention_days * 24 * 60 * 60);

        // Iterate through directory entries
        for entry in fs::read_dir(parent_dir).map_err(GrokError::Io)? {
            let entry = entry.map_err(GrokError::Io)?;
            let path = entry.path();
            
            // Check if this is a backup file for our original file
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&format!("{}.", file_name)) && name.ends_with(".bak") {
                    // Check file age
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age > retention_duration {
                                    // Remove old backup
                                    if let Err(e) = fs::remove_file(&path) {
                                        eprintln!("Warning: Failed to remove old backup {}: {}", 
                                                 path.display(), e);
                                    } else {
                                        removed.push(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !removed.is_empty() && std::env::var("DEBUG_API").is_ok() {
            use colored::*;
            eprintln!(
                "{}: Cleaned up {} old backup(s)",
                "BACKUP".blue().bold(),
                removed.len().to_string().yellow()
            );
        }

        Ok(removed)
    }

    /// Get all backups for a file
    pub fn list_backups(&self, original_file: &Path) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();
        let parent_dir = original_file.parent()
            .ok_or_else(|| GrokError::InvalidInput("File has no parent directory".to_string()))?;

        let file_name = original_file.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GrokError::InvalidInput("Invalid file name".to_string()))?;

        // Iterate through directory entries
        for entry in fs::read_dir(parent_dir).map_err(GrokError::Io)? {
            let entry = entry.map_err(GrokError::Io)?;
            let path = entry.path();
            
            // Check if this is a backup file for our original file
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&format!("{}.", file_name)) && name.ends_with(".bak") {
                    // Extract timestamp from filename
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let timestamp = modified.duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
                                .map(|dt| dt.with_timezone(&Local))
                                .unwrap_or_else(Local::now);
                            
                            backups.push(BackupInfo {
                                path,
                                created: datetime,
                                size: metadata.len(),
                            });
                        }
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created.cmp(&a.created));

        Ok(backups)
    }
}

/// Information about a backup file
#[derive(Debug)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub created: DateTime<Local>,
    pub size: u64,
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let manager = BackupManager::new(Some(7));
        let backup_path = manager.create_backup(&test_file).unwrap();

        assert!(backup_path.exists());
        assert!(backup_path.to_str().unwrap().contains(".bak"));
        
        // Verify content
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, "test content");
    }

    #[test]
    fn test_backup_pattern() {
        let manager = BackupManager::new(Some(7));
        let test_path = Path::new("/tmp/test.rs");
        let backup_path = manager.generate_backup_path(test_path);
        
        let backup_name = backup_path.file_name().unwrap().to_str().unwrap();
        assert!(backup_name.starts_with("test.rs."));
        assert!(backup_name.ends_with(".bak"));
    }

    // TODO: Add tests for cleanup functionality
} 