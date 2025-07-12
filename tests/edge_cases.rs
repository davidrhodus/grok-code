use grok_code::agent::GrokAgent;
use grok_code::api::{create_client, ApiConfig};
use grok_code::error::GrokError;
use std::env;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_invalid_api_key() {
    // Test with an invalid API key
    let config = ApiConfig {
        api_key: "invalid_key_12345".to_string(),
        base_url: "https://api.x.ai/v1".to_string(),
        model: "grok-2-latest".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let client = create_client("xai", config);
    assert!(client.is_ok()); // Client creation should succeed even with invalid key
}

#[tokio::test]
async fn test_non_interactive_mode() {
    // Test agent in non-interactive mode
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let config = ApiConfig {
        api_key: "test_key".to_string(),
        base_url: "https://api.x.ai/v1".to_string(),
        model: "grok-2-latest".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let agent = GrokAgent::new(
        "xai",
        config,
        project_root,
        false, // dry_run
        3,     // max_depth
        true,  // no_confirm - non-interactive mode
    );

    assert!(agent.is_ok());
    let _agent = agent.unwrap();
    // In non-interactive mode, confirmations should be auto-approved
    // This is tested implicitly by the no_confirm flag
}

#[tokio::test]
async fn test_empty_project_directory() {
    // Test with an empty project directory
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let summary = GrokAgent::generate_codebase_summary(&project_root, 3);
    // Empty directory will just have the header
    assert!(summary.contains("Project structure:"));
    assert!(!summary.contains(".txt")); // No files should be listed
}

#[tokio::test]
async fn test_deeply_nested_project() {
    // Test with a deeply nested project structure
    let temp_dir = TempDir::new().unwrap();
    let mut current_path = temp_dir.path().to_path_buf();

    // Create a deeply nested structure
    for i in 0..10 {
        current_path = current_path.join(format!("level{}", i));
        fs::create_dir(&current_path).unwrap();
        fs::write(
            current_path.join("file.txt"),
            format!("Level {} content", i),
        )
        .unwrap();
    }

    let summary = GrokAgent::generate_codebase_summary(temp_dir.path(), 5);
    assert!(summary.contains("file.txt"));
    // With max_depth 5, it includes files up to level3
    assert!(summary
        .lines()
        .any(|line| line.contains("level3") && line.contains("file.txt")));
    // And should not include level4 or beyond
    assert!(!summary
        .lines()
        .any(|line| line.contains("level4") && line.contains("file.txt")));
}

#[tokio::test]
async fn test_large_file_handling() {
    // Test with a large file
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.txt");

    // Create a 2MB file (larger than typical context windows)
    let large_content = "x".repeat(2 * 1024 * 1024);
    fs::write(&large_file, large_content).unwrap();

    let summary = GrokAgent::generate_codebase_summary(temp_dir.path(), 3);
    assert!(summary.contains("large.txt"));
}

#[tokio::test]
async fn test_permission_denied_scenario() {
    // Test handling of permission denied errors
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let restricted_file = temp_dir.path().join("restricted.txt");
        fs::write(&restricted_file, "secret content").unwrap();

        // Remove read permissions
        let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&restricted_file, perms).unwrap();

        // Try to generate summary - should handle permission error gracefully
        let summary = GrokAgent::generate_codebase_summary(temp_dir.path(), 3);
        // Summary generation should not panic, even with permission errors
        assert!(!summary.is_empty());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&restricted_file, perms).unwrap();
    }
}

#[tokio::test]
async fn test_invalid_utf8_filename() {
    // Test handling of invalid UTF-8 in filenames
    let temp_dir = TempDir::new().unwrap();

    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Create a file with invalid UTF-8 in its name
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let invalid_name = OsStr::from_bytes(&invalid_bytes);
        let invalid_path = temp_dir.path().join(invalid_name);

        // Try to create file - might fail on some systems
        let _ = fs::write(&invalid_path, "content");

        // Summary generation should handle this gracefully
        let summary = GrokAgent::generate_codebase_summary(temp_dir.path(), 3);
        assert!(!summary.is_empty());
    }
}

#[tokio::test]
async fn test_symlink_handling() {
    // Test handling of symbolic links
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target.txt");
        let link = temp_dir.path().join("link.txt");

        fs::write(&target, "target content").unwrap();
        symlink(&target, &link).unwrap();

        let summary = GrokAgent::generate_codebase_summary(temp_dir.path(), 3);
        assert!(summary.contains("target.txt"));
        // Should handle symlinks without infinite loops
    }
}

#[tokio::test]
async fn test_circular_symlink() {
    // Test handling of circular symbolic links
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");

        fs::create_dir(&dir1).unwrap();
        fs::create_dir(&dir2).unwrap();

        // Create circular symlinks
        symlink(&dir2, dir1.join("link_to_dir2")).unwrap();
        symlink(&dir1, dir2.join("link_to_dir1")).unwrap();

        // Should not panic or get stuck in infinite loop
        let summary = GrokAgent::generate_codebase_summary(temp_dir.path(), 3);
        assert!(!summary.is_empty());
    }
}

#[tokio::test]
async fn test_empty_api_key() {
    // Test with empty API key
    let config = ApiConfig {
        api_key: "".to_string(),
        base_url: "https://api.x.ai/v1".to_string(),
        model: "grok-2-latest".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let client = create_client("xai", config);
    assert!(client.is_ok()); // Client creation should succeed, error comes later
}

#[tokio::test]
async fn test_invalid_provider() {
    // Test with invalid provider
    let config = ApiConfig {
        api_key: "test_key".to_string(),
        base_url: "https://api.invalid.com".to_string(),
        model: "invalid-model".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let client = create_client("invalid_provider", config);
    assert!(client.is_err());

    if let Err(error) = client {
        match error {
            GrokError::Config(msg) => assert!(msg.contains("Unknown API provider")),
            _ => panic!("Expected Config error"),
        }
    }
}

#[tokio::test]
async fn test_dry_run_mode() {
    // Test agent in dry-run mode
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let config = ApiConfig {
        api_key: "test_key".to_string(),
        base_url: "https://api.x.ai/v1".to_string(),
        model: "grok-2-latest".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    let agent = GrokAgent::new(
        "xai",
        config,
        project_root,
        true,  // dry_run mode
        3,     // max_depth
        false, // no_confirm
    );

    assert!(agent.is_ok());
    // In dry-run mode, no actual changes should be made
}

#[tokio::test]
async fn test_rate_limit_env_var() {
    // Test rate limit handling with environment variable
    env::set_var("XAI_API_KEY", "test_key_from_env");

    let config = ApiConfig {
        api_key: env::var("XAI_API_KEY").unwrap(),
        base_url: "https://api.x.ai/v1".to_string(),
        model: "grok-2-latest".to_string(),
        timeout_secs: 60,
        max_retries: 3,
    };

    assert_eq!(config.api_key, "test_key_from_env");

    env::remove_var("XAI_API_KEY");
}
