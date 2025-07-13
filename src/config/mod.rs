pub mod agents_config;
pub mod helper;
pub mod rules_config;
pub mod triggers_config;
pub mod web_ui_config;

use crate::config::agents_config::AgentsConfig;
use crate::config::rules_config::Rule;
use crate::config::triggers_config::Trigger;
use crate::config::web_ui_config::WebUIConfig;
use anyhow::Result;
use serde::Deserialize;
use std::convert::TryFrom;

/// Main configuration structure matching config.yaml format
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub web_ui: WebUIConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
}

impl Config {
    /// Load configuration from YAML file
    pub fn from_file(config_path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", config_path, e))?;

        let config: Config = serde_yml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML config: {}", e))?;

        Ok(config)
    }


    /// Parse triggers from config
    pub fn parse_triggers(&self) -> Result<Vec<Trigger>> {
        let mut triggers = Vec::new();
        for trigger_config in &self.agents.triggers {
            let trigger = Trigger::try_from(trigger_config.clone())?;
            triggers.push(trigger);
        }
        Ok(triggers)
    }

    /// Parse rules from config
    pub fn parse_rules(&self) -> Result<Vec<Rule>> {
        let mut rules = Vec::new();
        for rule_config in &self.agents.rules {
            let rule = Rule::try_from(rule_config.clone())?;
            rules.push(rule);
        }
        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_file() {
        // Test loading a basic config file
        let config_path = "examples/basic/config.yaml";
        let config = Config::from_file(config_path).unwrap();

        // Test web_ui section
        assert!(config.web_ui.enabled);
        assert_eq!(config.web_ui.host, "localhost");
        assert_eq!(config.web_ui.base_port, 9990);

        // Test agents section
        assert_eq!(config.agents.pool, 1);
        assert!(!config.agents.triggers.is_empty());
        assert!(!config.agents.rules.is_empty());
    }
}
