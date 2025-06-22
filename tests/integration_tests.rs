// Integration tests for rule-agents binary
// Since this is now a binary-only project, these tests verify the binary functionality
// through command line interface testing

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_binary_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rule-agents"));
    assert!(stdout.contains("YAML-driven agent auto-control system"));
}

#[test]
fn test_binary_show_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "show", "--config", "config.yaml"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Loaded"));
    assert!(stdout.contains("rules"));
}

#[test]
fn test_binary_test_command() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "test",
            "--config",
            "config.yaml",
            "--capture",
            "Do you want to proceed",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Input:"));
    assert!(stdout.contains("Result:"));
}

#[test]
fn test_binary_with_invalid_rules_file() {
    let output = Command::new("cargo")
        .args(["run", "--", "show", "--config", "nonexistent.yaml"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Check that there's an error about the missing file
    assert!(stderr.contains("Error:") || stderr.contains("Failed"));
}

#[test]
fn test_binary_with_custom_rules() {
    let yaml_content = r#"
rules:
  - priority: 10
    pattern: "test pattern"
    action: "send_keys"
    keys: ["test", "\r"]
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", yaml_content).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "test",
            "--config",
            temp_file.path().to_str().unwrap(),
            "--capture",
            "test pattern",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Input:"));
    assert!(stdout.contains("test pattern"));
}

#[test]
fn test_binary_version() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rule-agents"));
}
