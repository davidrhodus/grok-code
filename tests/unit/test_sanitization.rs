#[cfg(test)]
mod tests {
    use grok_code::tools::{Tool, ToolContext, RunShellCommand, ReadFile};
    use serde_json::json;
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    fn create_test_context(temp_dir: &TempDir) -> ToolContext {
        ToolContext {
            project_root: temp_dir.path().to_path_buf(),
            dry_run: false,
            no_confirm: true, // Auto-confirm for tests
            git_repo: None,
        }
    }
    
    #[test]
    fn test_shell_command_sanitization() {
        let temp_dir = TempDir::new().unwrap();
        let context = create_test_context(&temp_dir);
        let tool = RunShellCommand;
        
        // Test dangerous commands are blocked
        let dangerous_commands = vec![
            "rm -rf /",
            "rm -rf /*",
            ":(){ :|:& };:",
            "dd if=/dev/zero of=/dev/sda",
            "> /etc/passwd",
        ];
        
        for cmd in dangerous_commands {
            let args = json!({ "command": cmd });
            let result = tool.execute(&args, &context);
            assert!(result.contains("Error:"), "Command '{}' should be blocked", cmd);
            assert!(!result.contains("Command executed"), "Command '{}' should not execute", cmd);
        }
        
        // Test safe commands work
        let safe_commands = vec![
            "echo 'hello world'",
            "ls",
            "pwd",
        ];
        
        for cmd in safe_commands {
            let args = json!({ "command": cmd });
            let result = tool.execute(&args, &context);
            assert!(!result.contains("Error: Dangerous"), "Command '{}' should be allowed", cmd);
        }
    }
    
    #[test]
    fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let context = create_test_context(&temp_dir);
        let tool = ReadFile;
        
        // Test path traversal attempts are blocked
        let malicious_paths = vec![
            "../../../etc/passwd",
            "../../../../../../etc/shadow",
            "../..",
            "test/../../../etc/hosts",
        ];
        
        for path in malicious_paths {
            let args = json!({ "path": path });
            let result = tool.execute(&args, &context);
            assert!(result.contains("Error:"), "Path '{}' should be blocked", path);
        }
        
        // Test normal paths work
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
        let args = json!({ "path": "test.txt" });
        let result = tool.execute(&args, &context);
        assert_eq!(result, "test content");
    }
    
    #[test] 
    fn test_null_byte_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let context = create_test_context(&temp_dir);
        let tool = ReadFile;
        
        // Test null bytes are blocked
        let args = json!({ "path": "test\0file.txt" });
        let result = tool.execute(&args, &context);
        assert!(result.contains("Error:"));
        assert!(result.contains("null bytes"));
    }
} 