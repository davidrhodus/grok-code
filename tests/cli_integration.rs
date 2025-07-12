use std::env;
use std::process::Command;

/// Test that the CLI runs and shows help
#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("grok-code"));
    assert!(stdout.contains("coding agent"));
}

/// Test that the check command works
#[test]
fn test_check_command_without_api_key() {
    // Temporarily unset any API keys
    env::remove_var("XAI_API_KEY");
    env::remove_var("OPENAI_API_KEY");

    let output = Command::new("cargo")
        .args(["run", "--", "check"])
        .output()
        .expect("Failed to execute process");

    // Should fail without API key
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("API key is required"));
}

/// Test dry-run mode
#[test]
fn test_dry_run_mode() {
    env::remove_var("XAI_API_KEY");
    env::remove_var("OPENAI_API_KEY");

    let output = Command::new("cargo")
        .args(["run", "--", "--dry-run", "--help"])
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());
}

/// Test max-depth argument parsing
#[test]
fn test_max_depth_argument() {
    let output = Command::new("cargo")
        .args(["run", "--", "--max-depth", "5", "--help"])
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());
}
