use super::utils::{sanitize_commit_message, sanitize_path};
use super::{Tool, ToolContext};
use crate::backup::BackupManager;
use git2::{IndexAddOption, Signature};
use regex::Regex;
use serde_json::{json, Value as JsonValue};
use std::env;
use std::fs;

/// Tool for creating git commits
pub struct CreateCommit;

impl Tool for CreateCommit {
    fn name(&self) -> &'static str {
        "create_commit"
    }

    fn description(&self) -> &'static str {
        "Create a git commit with staged changes."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string", "description": "The commit message."}
            },
            "required": ["message"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let message = match args["message"].as_str() {
            Some(msg) => match sanitize_commit_message(msg) {
                Ok(m) => m,
                Err(e) => return format!("Error: {e}"),
            },
            None => return "Error: 'message' parameter is required".to_string(),
        };

        if let Some(repo) = &context.git_repo {
            if !context.confirm_action("create git commit") {
                return "Commit not confirmed.".to_string();
            }

            if context.dry_run {
                return "Dry-run: Would create commit.".to_string();
            }

            let mut index = match repo.index() {
                Ok(i) => i,
                Err(e) => return format!("Error: {e}"),
            };

            if let Err(e) = index.add_all(["."].iter(), IndexAddOption::DEFAULT, None) {
                return format!("Error staging: {e}");
            }

            if let Err(e) = index.write() {
                return format!("Error writing index: {e}");
            }

            let tree_id = match index.write_tree() {
                Ok(id) => id,
                Err(e) => return format!("Error writing tree: {e}"),
            };

            let tree = match repo.find_tree(tree_id) {
                Ok(t) => t,
                Err(e) => return format!("Error finding tree: {e}"),
            };

            let parent = match repo.head() {
                Ok(head) => match head.peel_to_commit() {
                    Ok(c) => c,
                    Err(e) => return format!("Error: {e}"),
                },
                Err(_) => return "No parent commit.".to_string(),
            };

            let sig = match Signature::now("Grok Code", "grok@code.com") {
                Ok(s) => s,
                Err(e) => return format!("Error creating git signature: {e}"),
            };

            match repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent]) {
                Ok(_) => "Commit successful.".to_string(),
                Err(e) => format!("Error committing: {e}"),
            }
        } else {
            "No git repo found.".to_string()
        }
    }
}

/// Tool for submitting pull requests
pub struct SubmitPR;

impl Tool for SubmitPR {
    fn name(&self) -> &'static str {
        "submit_pr"
    }

    fn description(&self) -> &'static str {
        "Submit a pull request to GitHub. Requires GITHUB_TOKEN, GITHUB_REPO (owner/repo) env vars."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "title": {"type": "string", "description": "PR title."},
                "body": {"type": "string", "description": "PR body."},
                "base": {"type": "string", "description": "Base branch.", "default": "main"},
                "head": {"type": "string", "description": "Head branch."}
            },
            "required": ["title", "head"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let title = match args["title"].as_str() {
            Some(t) => {
                let t = t.trim();
                if t.is_empty() {
                    return "Error: PR title cannot be empty".to_string();
                }
                t
            }
            None => return "Error: 'title' parameter is required".to_string(),
        };

        let body = args["body"].as_str().unwrap_or("").trim();
        let base = args["base"].as_str().unwrap_or("main").trim();
        let head = match args["head"].as_str() {
            Some(h) => {
                let h = h.trim();
                if h.is_empty() {
                    return "Error: 'head' branch cannot be empty".to_string();
                }
                h
            }
            None => return "Error: 'head' parameter is required".to_string(),
        };

        let _token = match env::var("GITHUB_TOKEN") {
            Ok(t) => t,
            Err(_) => return "GITHUB_TOKEN env var required.".to_string(),
        };

        let repo_str = match env::var("GITHUB_REPO") {
            Ok(r) => r,
            Err(_) => return "GITHUB_REPO env var required (e.g., owner/repo).".to_string(),
        };

        if !context.confirm_action("submit PR to GitHub") {
            return "PR submission not confirmed.".to_string();
        }

        if context.dry_run {
            return "Dry-run: Would submit PR.".to_string();
        }

        let url = format!("https://api.github.com/repos/{repo_str}/pulls");
        let _pr_body = json!({
            "title": title,
            "body": body,
            "base": base,
            "head": head
        });

        // TODO: Implement actual GitHub API request
        // TODO: Handle authentication properly
        // TODO: Support draft PRs and PR templates
        // TODO: Add support for reviewers and labels
        format!("Would submit PR to {url} with title: {title}")
    }
}

/// Tool for resolving merge conflicts
pub struct ResolveMergeConflict;

impl Tool for ResolveMergeConflict {
    fn name(&self) -> &'static str {
        "resolve_merge_conflict"
    }

    fn description(&self) -> &'static str {
        "Intelligently resolve git merge conflicts with various strategies."
    }

    fn parameters(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the file with merge conflicts."},
                "strategy": {
                    "type": "string",
                    "description": "Resolution strategy: 'ours', 'theirs', 'both', 'interactive', or 'auto' (default)",
                    "default": "auto"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, args: &JsonValue, context: &ToolContext<'_>) -> String {
        let path_str = match args["path"].as_str() {
            Some(p) => p,
            None => return "Error: 'path' parameter is required".to_string(),
        };

        let strategy = args["strategy"].as_str().unwrap_or("auto");

        let path = match sanitize_path(path_str, &context.project_root) {
            Ok(p) => p,
            Err(e) => return format!("Error: {e}"),
        };

        if !context.confirm_action(&format!(
            "resolve merge conflicts in {} using '{}' strategy",
            path.display(),
            strategy
        )) {
            return "Merge resolution not confirmed.".to_string();
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                // Parse merge conflicts
                let conflict_pattern =
                    Regex::new(r"(?s)<<<<<<< ([^\n]+)\n(.*?)\n=======\n(.*?)\n>>>>>>> ([^\n]+)")
                        .unwrap();

                let mut conflicts = Vec::new();
                let mut conflict_count = 0;

                for cap in conflict_pattern.captures_iter(&content) {
                    conflict_count += 1;
                    conflicts.push(MergeConflict {
                        full_match: cap[0].to_string(),
                        ours_branch: cap[1].to_string(),
                        ours_content: cap[2].to_string(),
                        theirs_branch: cap[4].to_string(),
                        theirs_content: cap[3].to_string(),
                    });
                }

                if conflict_count == 0 {
                    return "No merge conflicts found in the file.".to_string();
                }

                if context.dry_run {
                    let mut preview = format!(
                        "Dry-run: Found {} conflict(s) in {}\n",
                        conflict_count,
                        path.display()
                    );
                    preview.push_str(&format!("Would resolve using '{strategy}' strategy:\n\n"));

                    for (i, conflict) in conflicts.iter().enumerate() {
                        preview.push_str(&format!(
                            "Conflict #{}: {} vs {}\n",
                            i + 1,
                            conflict.ours_branch,
                            conflict.theirs_branch
                        ));
                        match strategy {
                            "ours" => preview.push_str("  → Would keep our version\n"),
                            "theirs" => preview.push_str("  → Would keep their version\n"),
                            "both" => preview.push_str("  → Would keep both versions\n"),
                            "auto" => {
                                let resolution = auto_resolve_conflict(conflict);
                                preview
                                    .push_str(&format!("  → Auto-resolution: {}\n", resolution.1));
                            }
                            _ => preview.push_str("  → Would require manual resolution\n"),
                        }
                    }

                    return preview;
                }

                // Apply resolution
                let mut resolved_content = content.clone();

                for conflict in conflicts.iter() {
                    let replacement = match strategy {
                        "ours" => conflict.ours_content.clone(),
                        "theirs" => conflict.theirs_content.clone(),
                        "both" => format!("{}\n{}", conflict.ours_content, conflict.theirs_content),
                        "auto" => auto_resolve_conflict(conflict).0,
                        _ => {
                            return format!(
                                "Unknown strategy '{strategy}'. Use 'ours', 'theirs', 'both', or 'auto'."
                            );
                        }
                    };

                    resolved_content = resolved_content.replace(&conflict.full_match, &replacement);
                }

                // Create backup
                let backup_manager = BackupManager::new(None);
                let backup_path = match backup_manager.create_backup(&path) {
                    Ok(path) => path,
                    Err(e) => return format!("Error creating backup: {e}"),
                };

                match fs::write(&path, resolved_content) {
                    Ok(_) => {
                        format!("Successfully resolved {} conflict(s) using '{}' strategy. Original backed up to {}", 
                                conflict_count, strategy, backup_path.display())
                    }
                    Err(e) => format!("Error writing resolved file: {e}"),
                }
            }
            Err(e) => format!("Error reading file: {e}"),
        }
    }
}

#[derive(Debug)]
struct MergeConflict {
    full_match: String,
    ours_branch: String,
    ours_content: String,
    theirs_branch: String,
    theirs_content: String,
}

/// Intelligently resolve a conflict based on content analysis
fn auto_resolve_conflict(conflict: &MergeConflict) -> (String, String) {
    let ours = &conflict.ours_content;
    let theirs = &conflict.theirs_content;

    // If one side is empty, take the non-empty one
    if ours.trim().is_empty() && !theirs.trim().is_empty() {
        return (
            theirs.clone(),
            "Chose non-empty version (theirs)".to_string(),
        );
    }
    if theirs.trim().is_empty() && !ours.trim().is_empty() {
        return (ours.clone(), "Chose non-empty version (ours)".to_string());
    }

    // If both are identical, just use one
    if ours == theirs {
        return (ours.clone(), "Both versions identical".to_string());
    }

    // Check for common patterns

    // Import statements - usually want both
    if (ours.contains("import ") || ours.contains("use "))
        && (theirs.contains("import ") || theirs.contains("use "))
    {
        // Merge imports, removing duplicates
        let mut lines: Vec<&str> = ours.lines().chain(theirs.lines()).collect();
        lines.sort();
        lines.dedup();
        return (lines.join("\n"), "Merged import statements".to_string());
    }

    // Version numbers - take the higher one
    let version_regex = Regex::new(r"(\d+)\.(\d+)\.(\d+)").unwrap();
    if let (Some(ours_ver), Some(theirs_ver)) =
        (version_regex.captures(ours), version_regex.captures(theirs))
    {
        let ours_nums: Vec<u32> = vec![
            ours_ver[1].parse().unwrap_or(0),
            ours_ver[2].parse().unwrap_or(0),
            ours_ver[3].parse().unwrap_or(0),
        ];
        let theirs_nums: Vec<u32> = vec![
            theirs_ver[1].parse().unwrap_or(0),
            theirs_ver[2].parse().unwrap_or(0),
            theirs_ver[3].parse().unwrap_or(0),
        ];

        if ours_nums > theirs_nums {
            return (
                ours.clone(),
                "Chose higher version number (ours)".to_string(),
            );
        } else {
            return (
                theirs.clone(),
                "Chose higher version number (theirs)".to_string(),
            );
        }
    }

    // Comments - if one has more detailed comments, prefer that
    let ours_comment_lines = ours
        .lines()
        .filter(|l| l.trim().starts_with("//") || l.trim().starts_with("#"))
        .count();
    let theirs_comment_lines = theirs
        .lines()
        .filter(|l| l.trim().starts_with("//") || l.trim().starts_with("#"))
        .count();

    if ours_comment_lines > theirs_comment_lines * 2 {
        return (
            ours.clone(),
            "Chose version with more comments (ours)".to_string(),
        );
    } else if theirs_comment_lines > ours_comment_lines * 2 {
        return (
            theirs.clone(),
            "Chose version with more comments (theirs)".to_string(),
        );
    }

    // If one is significantly longer, it might have more implementation
    if ours.len() > theirs.len() * 2 {
        return (
            ours.clone(),
            "Chose more complete implementation (ours)".to_string(),
        );
    } else if theirs.len() > ours.len() * 2 {
        return (
            theirs.clone(),
            "Chose more complete implementation (theirs)".to_string(),
        );
    }

    // Default: combine both with clear markers
    let combined = format!(
        "// AUTO-MERGED: Kept both versions\n// === Original ({}): ===\n{}\n// === Alternative ({}): ===\n{}",
        conflict.ours_branch, ours, conflict.theirs_branch, theirs
    );
    (combined, "Kept both versions with markers".to_string())
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
        }
    }

    #[test]
    fn test_auto_resolve_empty_conflict() {
        let conflict = MergeConflict {
            full_match: String::new(),
            ours_branch: "main".to_string(),
            ours_content: "".to_string(),
            theirs_branch: "feature".to_string(),
            theirs_content: "Some content".to_string(),
        };

        let (resolved, reason) = auto_resolve_conflict(&conflict);
        assert_eq!(resolved, "Some content");
        assert!(reason.contains("non-empty"));
    }

    #[test]
    fn test_auto_resolve_identical_conflict() {
        let conflict = MergeConflict {
            full_match: String::new(),
            ours_branch: "main".to_string(),
            ours_content: "Same content".to_string(),
            theirs_branch: "feature".to_string(),
            theirs_content: "Same content".to_string(),
        };

        let (resolved, reason) = auto_resolve_conflict(&conflict);
        assert_eq!(resolved, "Same content");
        assert!(reason.contains("identical"));
    }

    #[test]
    fn test_auto_resolve_import_conflict() {
        let conflict = MergeConflict {
            full_match: String::new(),
            ours_branch: "main".to_string(),
            ours_content: "import foo\nimport bar".to_string(),
            theirs_branch: "feature".to_string(),
            theirs_content: "import bar\nimport baz".to_string(),
        };

        let (resolved, reason) = auto_resolve_conflict(&conflict);
        assert!(resolved.contains("import bar"));
        assert!(resolved.contains("import baz"));
        assert!(resolved.contains("import foo"));
        assert!(reason.contains("Merged import"));
    }

    #[test]
    fn test_auto_resolve_version_conflict() {
        let conflict = MergeConflict {
            full_match: String::new(),
            ours_branch: "main".to_string(),
            ours_content: "version = \"1.2.3\"".to_string(),
            theirs_branch: "feature".to_string(),
            theirs_content: "version = \"1.3.0\"".to_string(),
        };

        let (resolved, reason) = auto_resolve_conflict(&conflict);
        assert_eq!(resolved, "version = \"1.3.0\"");
        assert!(reason.contains("higher version"));
    }

    #[test]
    fn test_merge_strategies() {
        let tool = ResolveMergeConflict;
        let context = create_test_context();

        // Create a test file with conflicts
        let test_content = r#"Some code before
<<<<<<< HEAD
Our version
=======
Their version
>>>>>>> feature
Some code after"#;

        let test_file = "/tmp/test_merge.txt";
        std::fs::write(test_file, test_content).unwrap();

        // Test "ours" strategy
        let args = json!({
            "path": test_file,
            "strategy": "ours"
        });

        let result = tool.execute(&args, &context);
        assert!(result.contains("Successfully resolved"));

        let resolved = std::fs::read_to_string(test_file).unwrap();
        assert!(resolved.contains("Our version"));
        assert!(!resolved.contains("Their version"));
        assert!(!resolved.contains("<<<<<<<"));

        // Clean up
        std::fs::remove_file(test_file).ok();
        std::fs::remove_file("/tmp/test_merge.bak").ok();
    }
}
