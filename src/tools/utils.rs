use regex::Regex;
use std::path::{Path, PathBuf};

/// Sanitize and validate file paths to prevent directory traversal attacks
pub fn sanitize_path(path: &str, root: &Path) -> Result<PathBuf, String> {
    // Remove any null bytes
    if path.contains('\0') {
        return Err("Path contains null bytes".to_string());
    }

    // Normalize the path
    let path = path.trim();

    // Check for suspicious patterns
    if path.contains("..") {
        // Allow .. only if the resolved path is still within the root
        let full_path = root.join(path);
        match full_path.canonicalize() {
            Ok(canonical) => {
                // Check if the canonical path is within the root
                match root.canonicalize() {
                    Ok(canonical_root) => {
                        if !canonical.starts_with(&canonical_root) {
                            return Err("Path traversal detected: path goes outside project root"
                                .to_string());
                        }
                    }
                    Err(_) => {
                        // If we can't canonicalize root, be conservative
                        return Err(
                            "Cannot validate path: unable to resolve project root".to_string()
                        );
                    }
                }
            }
            Err(_) => {
                // Path doesn't exist yet, do a simple check
                if path.contains("../") || path.starts_with("..") {
                    return Err("Path contains directory traversal patterns".to_string());
                }
            }
        }
    }

    Ok(root.join(path))
}

/// Sanitize shell commands to prevent injection attacks
pub fn sanitize_shell_command(command: &str) -> Result<String, String> {
    let command = command.trim();

    // Check for empty command
    if command.is_empty() {
        return Err("Command cannot be empty".to_string());
    }

    // List of dangerous patterns
    let dangerous_patterns = vec![
        // File system destruction
        ("rm -rf /", "Deleting root filesystem"),
        ("rm -rf /*", "Deleting everything in root"),
        ("rm -rf ~", "Deleting home directory"),
        ("rm -rf *", "Deleting everything in current directory"),
        ("chmod -R 777 /", "Making entire filesystem world-writable"),
        ("chmod 000 /", "Making root inaccessible"),
        // Disk operations
        ("dd if=/dev/zero of=/", "Overwriting disk with zeros"),
        (
            "dd if=/dev/random of=/",
            "Overwriting disk with random data",
        ),
        ("mkfs.", "Formatting filesystem"),
        // Fork bombs and resource exhaustion
        (":(){ :|:& };:", "Fork bomb"),
        (":(){:|:&};:", "Fork bomb (no spaces)"),
        ("fork while fork", "Fork bomb variant"),
        // Network attacks
        ("nc -l", "Opening network listener (potential backdoor)"),
        // System modification
        ("> /etc/passwd", "Overwriting password file"),
        ("> /etc/shadow", "Overwriting shadow password file"),
        ("echo > /proc/sys", "Modifying kernel parameters"),
    ];

    for (pattern, description) in &dangerous_patterns {
        if command.contains(pattern) {
            return Err(format!(
                "Dangerous command pattern detected: {pattern} - {description}"
            ));
        }
    }

    // Check for command chaining that might bypass checks
    let chain_operators = vec!["&&", "||", ";", "|", "`", "$(", "<(", ">("];
    let mut has_chaining = false;
    for op in &chain_operators {
        if command.contains(op) {
            has_chaining = true;
            break;
        }
    }

    if has_chaining {
        // If command has chaining, do more thorough checks
        // Split by common operators and check each part
        let parts: Vec<&str> = command.split([';', '&', '|']).collect();
        for part in parts {
            let part = part.trim();
            for (pattern, description) in &dangerous_patterns {
                if part.contains(pattern) {
                    return Err(format!(
                        "Dangerous pattern in command chain: {pattern} - {description}"
                    ));
                }
            }
        }
    }

    Ok(command.to_string())
}

/// Sanitize git branch names according to git's rules
pub fn sanitize_git_branch_name(name: &str) -> Result<String, String> {
    let name = name.trim();

    if name.is_empty() {
        return Err("Branch name cannot be empty".to_string());
    }

    // Git branch name rules:
    // 1. Cannot start with '-'
    if name.starts_with('-') {
        return Err("Branch name cannot start with '-'".to_string());
    }

    // 2. Cannot contain: space, ~, ^, :, ?, *, [, \, .., @{
    let invalid_chars = vec![" ", "~", "^", ":", "?", "*", "[", "\\", "..", "@{"];
    for invalid in invalid_chars {
        if name.contains(invalid) {
            return Err(format!("Branch name cannot contain '{invalid}'"));
        }
    }

    // 3. Cannot end with '/'
    if name.ends_with('/') {
        return Err("Branch name cannot end with '/'".to_string());
    }

    // 4. Cannot end with '.lock'
    if name.ends_with(".lock") {
        return Err("Branch name cannot end with '.lock'".to_string());
    }

    // 5. Cannot be '.' or '..'
    if name == "." || name == ".." {
        return Err("Branch name cannot be '.' or '..'".to_string());
    }

    // 6. Cannot contain consecutive dots
    if name.contains("..") {
        return Err("Branch name cannot contain consecutive dots".to_string());
    }

    Ok(name.to_string())
}

/// Sanitize commit messages
pub fn sanitize_commit_message(message: &str) -> Result<String, String> {
    let message = message.trim();

    if message.is_empty() {
        return Err("Commit message cannot be empty".to_string());
    }

    // Remove any null bytes
    if message.contains('\0') {
        return Err("Commit message contains null bytes".to_string());
    }

    // Limit length to prevent abuse
    const MAX_COMMIT_MESSAGE_LENGTH: usize = 1000;
    if message.len() > MAX_COMMIT_MESSAGE_LENGTH {
        return Err(format!(
            "Commit message too long (max {MAX_COMMIT_MESSAGE_LENGTH} characters)"
        ));
    }

    Ok(message.to_string())
}

/// Validate and sanitize regex patterns
pub fn sanitize_regex_pattern(pattern: &str) -> Result<String, String> {
    // Try to compile the regex to ensure it's valid
    match Regex::new(pattern) {
        Ok(_) => Ok(pattern.to_string()),
        Err(e) => Err(format!("Invalid regex pattern: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_sanitize_path() {
        let root = Path::new("/project");

        // Valid paths
        assert!(sanitize_path("src/main.rs", root).is_ok());
        assert!(sanitize_path("./test.txt", root).is_ok());
        assert!(sanitize_path("subdir/file.txt", root).is_ok());

        // Invalid paths
        assert!(sanitize_path("../../../etc/passwd", root).is_err());
        assert!(sanitize_path("/etc/passwd", root).is_ok()); // Absolute paths are joined with root
        assert!(sanitize_path("test\0file", root).is_err());
    }

    #[test]
    fn test_sanitize_shell_command() {
        // Valid commands
        assert!(sanitize_shell_command("ls -la").is_ok());
        assert!(sanitize_shell_command("cargo test").is_ok());
        assert!(sanitize_shell_command("echo 'hello world'").is_ok());

        // Invalid commands
        assert!(sanitize_shell_command("rm -rf /").is_err());
        assert!(sanitize_shell_command(":(){ :|:& };:").is_err());
        assert!(sanitize_shell_command("dd if=/dev/zero of=/dev/sda").is_err());
        assert!(sanitize_shell_command("").is_err());

        // Commands with chaining
        assert!(sanitize_shell_command("echo test && rm -rf /").is_err());
        assert!(sanitize_shell_command("ls | grep test").is_ok());
    }

    #[test]
    fn test_sanitize_git_branch_name() {
        // Valid names
        assert!(sanitize_git_branch_name("feature/new-feature").is_ok());
        assert!(sanitize_git_branch_name("bugfix-123").is_ok());
        assert!(sanitize_git_branch_name("main").is_ok());

        // Invalid names
        assert!(sanitize_git_branch_name("-feature").is_err());
        assert!(sanitize_git_branch_name("feature branch").is_err());
        assert!(sanitize_git_branch_name("feature/").is_err());
        assert!(sanitize_git_branch_name("feature.lock").is_err());
        assert!(sanitize_git_branch_name("..").is_err());
        assert!(sanitize_git_branch_name("").is_err());
    }

    #[test]
    fn test_sanitize_commit_message() {
        // Valid messages
        assert!(sanitize_commit_message("Fix bug in parser").is_ok());
        assert!(sanitize_commit_message("Add new feature\n\nDetailed description").is_ok());

        // Invalid messages
        assert!(sanitize_commit_message("").is_err());
        assert!(sanitize_commit_message("test\0message").is_err());
        assert!(sanitize_commit_message(&"x".repeat(1001)).is_err());
    }
}
