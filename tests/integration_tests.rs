use anyhow::Result;
use rule_agents::rule_engine::{load_rules, CmdKind};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

#[test]
fn test_load_basic_rules() -> Result<()> {
    let rules = load_rules(Path::new("examples/simple-rules.yaml"))?;
    assert!(!rules.is_empty());
    // Verify first rule has lowest priority number
    assert_eq!(rules[0].priority, 10);
    assert_eq!(rules[0].command, CmdKind::SolveIssue);
    Ok(())
}

#[test]
fn test_invalid_yaml() {
    let yaml_content = "invalid yaml content [";
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", yaml_content).unwrap();

    let result = load_rules(temp_file.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("parse"));
}

#[test]
fn test_invalid_regex() -> Result<()> {
    let yaml_content = r#"
rules:
  - priority: 10
    pattern: "[invalid"
    command: "resume"
    args: []
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let result = load_rules(temp_file.path());
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    println!("Error message: {}", error_msg);
    assert!(error_msg.contains("Failed to compile rule with pattern"));
    Ok(())
}

#[test]
fn test_unknown_command() -> Result<()> {
    let yaml_content = r#"
rules:
  - priority: 10
    pattern: "test"
    command: "unknown-command"
    args: []
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let result = load_rules(temp_file.path());
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    println!("Error message: {}", error_msg);
    assert!(error_msg.contains("Failed to compile rule with pattern"));
    Ok(())
}

#[test]
fn test_priority_sorting() -> Result<()> {
    let yaml_content = r#"
rules:
  - priority: 30
    pattern: "third"
    command: "resume"
    args: []
  - priority: 10
    pattern: "first"
    command: "solve-issue"
    args: []
  - priority: 20
    pattern: "second"
    command: "cancel"
    args: []
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let rules = load_rules(temp_file.path())?;
    assert_eq!(rules.len(), 3);
    assert_eq!(rules[0].priority, 10);
    assert_eq!(rules[1].priority, 20);
    assert_eq!(rules[2].priority, 30);
    Ok(())
}

#[test]
fn test_file_not_found() {
    let result = load_rules(Path::new("nonexistent.yaml"));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to read rules file"));
}
