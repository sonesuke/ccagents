use anyhow::Result;
use rule_agents::{RuleEngine, RuleFile};
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_load_rule_file() -> Result<()> {
    let yaml_content = r#"
version: "1.0"
name: "Test Rules"
rules:
  - name: "test-rule"
    description: "A test rule"
    trigger:
      type: "interval"
      seconds: 60
    actions:
      - type: "log"
        level: "info"
        message: "Test message"
"#;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", yaml_content)?;

    let rule_file = RuleFile::load(temp_file.path())?;
    assert_eq!(rule_file.version, "1.0");
    assert_eq!(rule_file.name, "Test Rules");
    assert_eq!(rule_file.rules.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_rule_engine_creation() -> Result<()> {
    let mut engine = RuleEngine::new();

    // Test that engine starts and stops without errors
    engine.start().await?;
    engine.stop().await?;

    Ok(())
}
