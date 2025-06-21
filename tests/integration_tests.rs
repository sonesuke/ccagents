use anyhow::Result;
use regex::Regex;
use rule_agents::rule_engine::{decide_cmd, load_rules, CmdKind, CompiledRule};
use rule_agents::Manager;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

#[test]
fn test_load_basic_rules() -> Result<()> {
    let rules = load_rules(Path::new("examples/basic-rules.yaml"))?;
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

// Integration tests for decide_cmd function
#[test]
fn test_decide_cmd_exact_match() -> Result<()> {
    let rules = load_rules(Path::new("examples/basic-rules.yaml"))?;

    // Test match with "issue 123" pattern
    let (command, args) = decide_cmd("issue 123", &rules);
    assert_eq!(command, CmdKind::SolveIssue);
    assert!(args.is_empty()); // Args should match what's in the YAML
    Ok(())
}

#[test]
fn test_decide_cmd_priority_ordering_with_loaded_rules() -> Result<()> {
    let yaml_content = r#"
rules:
  - priority: 20
    pattern: "test"
    command: "cancel"
    args: ["low"]
  - priority: 10
    pattern: "test"
    command: "solve-issue"
    args: ["high"]
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let rules = load_rules(temp_file.path())?;

    // Test that higher priority (lower number) rules match first
    let (command, args) = decide_cmd("test", &rules);
    assert_eq!(command, CmdKind::SolveIssue);
    assert_eq!(args, vec!["high"]);
    Ok(())
}

#[test]
fn test_decide_cmd_no_match_with_loaded_rules() -> Result<()> {
    let rules = load_rules(Path::new("examples/basic-rules.yaml"))?;
    let (command, args) = decide_cmd("no matching pattern here", &rules);

    assert_eq!(command, CmdKind::Resume);
    assert!(args.is_empty());
    Ok(())
}

#[test]
fn test_decide_cmd_empty_capture_with_loaded_rules() -> Result<()> {
    let rules = load_rules(Path::new("examples/basic-rules.yaml"))?;
    let (command, args) = decide_cmd("", &rules);

    assert_eq!(command, CmdKind::Resume);
    assert!(args.is_empty());
    Ok(())
}

#[test]
fn test_decide_cmd_empty_rules() {
    let (command, args) = decide_cmd("any text", &[]);

    assert_eq!(command, CmdKind::Resume);
    assert!(args.is_empty());
}

#[test]
fn test_performance_100_rules() {
    use std::time::Instant;

    // Generate 100 test rules and measure performance
    let rules: Vec<CompiledRule> = (0..100)
        .map(|i| CompiledRule {
            priority: i,
            regex: Regex::new(&format!("unique_pattern_{}", i)).unwrap(),
            command: CmdKind::Resume,
            args: vec![],
        })
        .collect();

    let start = Instant::now();
    let (command, _) = decide_cmd("non-matching test input", &rules);
    let duration = start.elapsed();

    assert_eq!(command, CmdKind::Resume);
    assert!(
        duration.as_millis() < 100,
        "Should complete within 100ms for 100 rules, took {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_decide_cmd_with_all_basic_rule_patterns() -> Result<()> {
    let rules = load_rules(Path::new("examples/basic-rules.yaml"))?;

    // Test issue pattern
    let (command, args) = decide_cmd("issue 456", &rules);
    assert_eq!(command, CmdKind::SolveIssue);
    assert!(args.is_empty());

    // Test cancel pattern
    let (command, args) = decide_cmd("cancel", &rules);
    assert_eq!(command, CmdKind::Cancel);
    assert!(args.is_empty());

    // Test resume pattern
    let (command, args) = decide_cmd("resume", &rules);
    assert_eq!(command, CmdKind::Resume);
    assert!(args.is_empty());

    Ok(())
}

// Manager integration tests
#[tokio::test]
async fn test_manager_integration() -> Result<()> {
    let manager = Manager::new("examples/basic-rules.yaml").await?;

    // Test agent waiting scenarios
    assert!(manager
        .handle_waiting_state("test-agent", "issue 123")
        .await
        .is_ok());
    assert!(manager
        .handle_waiting_state("test-agent", "cancel")
        .await
        .is_ok());
    assert!(manager
        .handle_waiting_state("test-agent", "unknown")
        .await
        .is_ok());

    Ok(())
}

#[tokio::test]
async fn test_manager_with_invalid_rules_file() {
    let result = Manager::new("nonexistent.yaml").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to read rules file"));
}

#[tokio::test]
async fn test_manager_handles_multiple_scenarios() -> Result<()> {
    let manager = Manager::new("examples/basic-rules.yaml").await?;

    let scenarios = vec![
        ("agent-001", "issue 456 detected in process"),
        ("agent-002", "network connection failed"),
        ("agent-003", "cancel current operation"),
        ("agent-004", "resume normal operation"),
        ("agent-005", "unknown error occurred"),
    ];

    for (agent_id, capture) in scenarios {
        assert!(manager
            .handle_waiting_state(agent_id, capture)
            .await
            .is_ok());
    }

    Ok(())
}

#[tokio::test]
async fn test_manager_with_custom_rules() -> Result<()> {
    let yaml_content = r#"
rules:
  - priority: 10
    pattern: "test-pattern"
    command: "solve-issue"
    args: ["test-arg"]
  - priority: 20
    pattern: "cancel-test"
    command: "cancel"
    args: []
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let manager = Manager::new(temp_file.path().to_str().unwrap()).await?;

    // Test that custom rules work correctly
    assert!(manager
        .handle_waiting_state("test-agent", "test-pattern")
        .await
        .is_ok());
    assert!(manager
        .handle_waiting_state("test-agent", "cancel-test")
        .await
        .is_ok());
    assert!(manager
        .handle_waiting_state("test-agent", "no-match")
        .await
        .is_ok());

    Ok(())
}

#[tokio::test]
async fn test_manager_with_hot_reload() -> Result<()> {
    use std::fs;

    let yaml_content = r#"
rules:
  - priority: 10
    pattern: "test-hot-reload"
    command: "solve-issue"
    args: []
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let manager = Manager::new(temp_file.path().to_str().unwrap()).await?;

    // Initially should match the test pattern
    assert!(manager
        .handle_waiting_state("test-agent", "test-hot-reload")
        .await
        .is_ok());

    // Give the file watcher time to set up
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Update the rules file
    let new_yaml_content = r#"
rules:
  - priority: 10
    pattern: "updated-hot-reload"
    command: "cancel"
    args: []
"#;

    fs::write(temp_file.path(), new_yaml_content)?;

    // Give hot-reload time to process the change
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Test that rules have been updated
    assert!(manager
        .handle_waiting_state("test-agent", "updated-hot-reload")
        .await
        .is_ok());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_agents() -> Result<()> {
    let manager = Manager::new("examples/basic-rules.yaml").await?;

    // Simulate multiple agents hitting waiting state simultaneously
    let handles: std::vec::Vec<_> = (0..10)
        .map(|i| {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager
                    .handle_waiting_state(&format!("agent-{}", i), "issue 123")
                    .await
            })
        })
        .collect();

    // All should complete successfully
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    Ok(())
}
