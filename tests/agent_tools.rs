use grok_code::tools::{ToolContext, ToolRegistry};
use std::fs;
use tempfile::TempDir;

fn create_test_context(temp_dir: &TempDir) -> ToolContext {
    ToolContext {
        project_root: temp_dir.path().to_path_buf(),
        dry_run: false,
        no_confirm: true,
        git_repo: None,
        tui_mode: false,
    }
}

#[test]
fn test_tool_registry_creation() {
    let registry = ToolRegistry::new();
    let tools = registry.get_tools();

    // Check that all expected tools are registered
    let tool_names: Vec<String> = tools.iter().map(|t| t.name().to_string()).collect();

    assert!(tool_names.contains(&"read_file".to_string()));
    assert!(tool_names.contains(&"write_file".to_string()));
    assert!(tool_names.contains(&"edit_file".to_string()));
    assert!(tool_names.contains(&"list_files".to_string()));
    assert!(tool_names.contains(&"run_shell_command".to_string()));
    assert!(tool_names.contains(&"search_codebase".to_string()));
    assert!(tool_names.contains(&"debug_code".to_string()));
    assert!(tool_names.contains(&"run_lint".to_string()));
    assert!(tool_names.contains(&"create_commit".to_string()));
    assert!(tool_names.contains(&"submit_pr".to_string()));
    assert!(tool_names.contains(&"resolve_merge_conflict".to_string()));
    assert!(tool_names.contains(&"analyze_log".to_string()));
    assert!(tool_names.contains(&"web_search".to_string()));
    assert!(tool_names.contains(&"create_jira_ticket".to_string()));
}

#[test]
fn test_file_operations_integration() {
    let temp_dir = TempDir::new().unwrap();
    let context = create_test_context(&temp_dir);
    let registry = ToolRegistry::new();

    // Test write_file
    let write_args = r#"{"path": "test.txt", "content": "Hello, World!"}"#;
    let write_result = registry.execute_tool("write_file", write_args, &context);
    assert!(write_result.contains("File written successfully"));

    // Verify file exists
    let file_path = temp_dir.path().join("test.txt");
    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "Hello, World!");

    // Test read_file
    let read_args = r#"{"path": "test.txt"}"#;
    let read_result = registry.execute_tool("read_file", read_args, &context);
    assert_eq!(read_result.trim(), "Hello, World!");

    // Test list_files
    let list_args = r#"{"path": "."}"#;
    let list_result = registry.execute_tool("list_files", list_args, &context);
    assert!(list_result.contains("test.txt"));
}

#[test]
fn test_edit_file_integration() {
    let temp_dir = TempDir::new().unwrap();
    let context = create_test_context(&temp_dir);
    let registry = ToolRegistry::new();

    // Create initial file
    let file_path = temp_dir.path().join("edit_test.txt");
    fs::write(&file_path, "Line 1\nLine 2\nLine 3\n").unwrap();

    // Test edit_file
    let edit_args = r#"{"path": "edit_test.txt", "start_line": 2, "end_line": 2, "new_content": "Modified Line 2\n"}"#;
    let edit_result = registry.execute_tool("edit_file", edit_args, &context);
    assert!(edit_result.contains("File edited successfully"));

    // Verify edit
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("Modified Line 2"));
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 3"));
}

#[test]
fn test_search_codebase_integration() {
    let temp_dir = TempDir::new().unwrap();
    let context = create_test_context(&temp_dir);
    let registry = ToolRegistry::new();

    // Create test files
    fs::write(
        temp_dir.path().join("test1.rs"),
        "fn main() {\n    println!(\"Hello\");\n}",
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("test2.rs"),
        "fn helper() {\n    // TODO: implement\n}",
    )
    .unwrap();
    fs::write(temp_dir.path().join("test.txt"), "This is not a rust file").unwrap();

    // Test search with pattern
    let search_args = r#"{"query": "fn"}"#;
    let search_result = registry.execute_tool("search_codebase", search_args, &context);

    assert!(search_result.contains("test1.rs"));
    assert!(search_result.contains("test2.rs"));
    // The tool only reports file names, not content
    assert!(search_result.contains("Found 'fn'"));
}

#[test]
fn test_shell_command_integration() {
    let temp_dir = TempDir::new().unwrap();
    let context = create_test_context(&temp_dir);
    let registry = ToolRegistry::new();

    // Test safe command
    let args = r#"{"command": "echo 'Hello from shell'"}"#;
    let result = registry.execute_tool("run_shell_command", args, &context);
    assert!(result.contains("Hello from shell"));

    // Test command with proper escaping
    let args = r#"{"command": "echo \"Test with quotes\""}"#;
    let result = registry.execute_tool("run_shell_command", args, &context);
    assert!(result.contains("Test with quotes"));
}

#[test]
fn test_tool_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let context = create_test_context(&temp_dir);
    let registry = ToolRegistry::new();

    // Test read non-existent file
    let args = r#"{"path": "nonexistent.txt"}"#;
    let result = registry.execute_tool("read_file", args, &context);
    assert!(result.contains("Error") || result.contains("not found"));

    // Test invalid JSON args
    let result = registry.execute_tool("read_file", "{invalid json}", &context);
    assert!(result.contains("Invalid arguments"));

    // Test non-existent tool
    let result = registry.execute_tool("fake_tool", "{}", &context);
    assert!(result.contains("Unknown tool"));
}

#[test]
fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    let mut context = create_test_context(&temp_dir);
    context.dry_run = true;
    let registry = ToolRegistry::new();

    // Test write_file in dry-run mode
    let args = r#"{"path": "dryrun_test.txt", "content": "Should not be written"}"#;
    let result = registry.execute_tool("write_file", args, &context);
    assert!(result.contains("Dry-run:"));

    // Verify file was not created
    let file_path = temp_dir.path().join("dryrun_test.txt");
    assert!(!file_path.exists());

    // Test shell command in dry-run mode
    let args = r#"{"command": "touch should_not_exist.txt"}"#;
    let result = registry.execute_tool("run_shell_command", args, &context);
    assert!(result.contains("Dry-run:") || result.contains("Would run"));
    assert!(!temp_dir.path().join("should_not_exist.txt").exists());
}

#[test]
fn test_git_operations() {
    let temp_dir = TempDir::new().unwrap();
    let context = create_test_context(&temp_dir);
    let registry = ToolRegistry::new();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    // Configure git for the test
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    // Create and stage a file
    fs::write(temp_dir.path().join("test.txt"), "Initial content").unwrap();
    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    // Make initial commit for the test
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    // Create a merge conflict file for testing
    let conflict_content = "Line 1\n<<<<<<< HEAD\nLine 2 from HEAD\n=======\nLine 2 from branch\n>>>>>>> branch\nLine 3";
    fs::write(temp_dir.path().join("conflict.txt"), conflict_content).unwrap();

    // Test resolve_merge_conflict with auto strategy
    let args = r#"{"path": "conflict.txt", "strategy": "auto"}"#;
    let result = registry.execute_tool("resolve_merge_conflict", args, &context);
    assert!(result.contains("Successfully resolved") || result.contains("No merge conflicts"));
}
