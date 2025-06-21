// Integration tests for rule-agents binary
// Since this is now a binary-only project, these tests verify the binary functionality
// through command line interface testing

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_binary_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
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
        .args(&["run", "--", "show", "--rules", "examples/basic-rules.yaml"])
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
        .args(&[
            "run",
            "--",
            "test",
            "--rules",
            "examples/basic-rules.yaml",
            "--capture",
            "issue 123",
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
        .args(&["run", "--", "show", "--rules", "nonexistent.yaml"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load rules"));
}

#[test]
fn test_binary_with_custom_rules() {
    let yaml_content = r#"
rules:
  - priority: 10
    pattern: "test pattern"
    command: "resume"
    args: []
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", yaml_content).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "test",
            "--rules",
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
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rule-agents"));
}
