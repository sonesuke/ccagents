pub mod agents;
pub mod loader;
pub mod rule;
pub mod terminal;
pub mod trigger;
pub mod types;
pub mod web_ui;

use crate::config::loader::load_config;
use crate::config::rule::Rule;
use crate::config::trigger::TriggerManager;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Config {
    trigger_manager: TriggerManager,
    rules: Arc<RwLock<Vec<Rule>>>,
    // Monitor configuration
    monitor_config: loader::MonitorConfig,
}

impl Config {
    pub async fn new(config_path: &str) -> Result<Self> {
        // Load initial configuration (triggers, rules, and monitor config)
        let (initial_triggers, initial_rules, monitor_config) =
            load_config(std::path::Path::new(config_path))?;
        let trigger_manager = TriggerManager::new(initial_triggers);
        let rules = Arc::new(RwLock::new(initial_rules));

        Ok(Config {
            trigger_manager,
            rules,
            monitor_config,
        })
    }

    /// Get monitor configuration
    pub fn get_monitor_config(&self) -> &loader::MonitorConfig {
        &self.monitor_config
    }

    /// Get trigger manager
    pub fn get_trigger_manager(&self) -> &TriggerManager {
        &self.trigger_manager
    }

    /// Get parsed rules directly
    pub fn get_rules(&self) -> Vec<Rule> {
        self.rules.try_read().map(|r| r.clone()).unwrap_or_default()
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field(
                "rules_count",
                &self.rules.try_read().map(|r| r.len()).unwrap_or(0),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_rules() {
        // Use existing diff-timeout-demo config
        let config_path = "examples/diff-timeout-demo/config.yaml";
        let config = Config::new(config_path).await.unwrap();

        // Test that we can get rules directly
        let rules = config.get_rules();

        // Should have some rules (the diff-timeout-demo config has rules)
        assert!(!rules.is_empty(), "Should have rules from config file");

        // Test that we can call it multiple times
        let rules2 = config.get_rules();
        assert_eq!(
            rules.len(),
            rules2.len(),
            "Should get same number of rules each time"
        );
    }
}
