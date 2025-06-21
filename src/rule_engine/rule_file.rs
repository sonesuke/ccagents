use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFile {
    pub version: String,
    pub name: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub description: String,
    pub trigger: Trigger,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    FileChange { patterns: Vec<String> },
    Interval { seconds: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    PatternMatch { pattern: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Log { level: String, message: String },
    Notify { channel: String },
    Command { command: String, timeout: u64 },
    CollectMetrics { metrics: Vec<String> },
}

impl RuleFile {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let rule_file = serde_yaml::from_str(&content)?;
        Ok(rule_file)
    }
}
