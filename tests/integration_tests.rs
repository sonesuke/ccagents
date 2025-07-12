// Integration tests for ccauto binary
// Since this is now a binary-only project, these tests verify the binary functionality
// through command line interface testing

use std::process::Command;

#[test]
fn test_binary_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ccauto"));
    assert!(stdout.contains("YAML-driven agent auto-control system"));
}

#[test]
fn test_binary_with_invalid_config_file() {
    let output = Command::new("cargo")
        .args(["run", "--", "--config", "nonexistent.yaml"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Check that there's an error about the missing file
    assert!(stderr.contains("Error:") || stderr.contains("Failed"));
}

#[test]
fn test_binary_version() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ccauto"));
}
