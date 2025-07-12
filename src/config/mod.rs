pub mod agents;
pub mod loader;
pub mod rule;
pub mod terminal;
pub mod trigger;
pub mod types;
pub mod web_ui;

use crate::config::loader::load_config;
use crate::config::rule::CompiledRule;
use crate::config::trigger::{Trigger, TriggerType};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for trigger system
pub struct TriggerConfig {
    entries: Arc<RwLock<Vec<Trigger>>>,
}

impl TriggerConfig {
    /// Get startup entries (on_start triggers)
    pub async fn get_on_start_entries(&self) -> Vec<Trigger> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|entry| entry.trigger == TriggerType::OnStart)
            .cloned()
            .collect()
    }

    /// Get periodic entries (periodic triggers)
    pub async fn get_periodic_entries(&self) -> Vec<Trigger> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|entry| matches!(entry.trigger, TriggerType::Periodic { .. }))
            .cloned()
            .collect()
    }
}

/// Detect if running in test environment
pub fn is_test_mode() -> bool {
    std::env::var("CARGO_TEST").is_ok()
        || cfg!(test)
        || std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::thread::current()
            .name()
            .is_some_and(|name| name.contains("test"))
        || std::env::args().any(|arg| arg.contains("test"))
        || std::env::current_exe()
            .map(|exe| {
                exe.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains("test")
            })
            .unwrap_or(false)
}

pub struct Config {
    entries: Arc<RwLock<Vec<Trigger>>>,
    rules: Arc<RwLock<Vec<CompiledRule>>>,
    test_mode: bool,
    // Monitor configuration
    monitor_config: loader::MonitorConfig,
}

impl Config {
    pub async fn new(config_path: &str) -> Result<Self> {
        // Load initial configuration (entries, rules, and monitor config)
        let (initial_entries, initial_rules, monitor_config) =
            load_config(std::path::Path::new(config_path))?;
        let entries = Arc::new(RwLock::new(initial_entries.clone()));
        let rules = Arc::new(RwLock::new(initial_rules));

        // In test environment, create a simple mock backend that always succeeds
        let is_test = is_test_mode();

        Ok(Config {
            entries,
            rules,
            test_mode: is_test,
            monitor_config,
        })
    }

    /// Get monitor configuration
    pub fn get_monitor_config(&self) -> &loader::MonitorConfig {
        &self.monitor_config
    }

    /// Get trigger configuration
    pub fn get_trigger_config(&self) -> TriggerConfig {
        TriggerConfig {
            entries: Arc::clone(&self.entries),
        }
    }

    /// Get compiled rules directly
    pub fn get_rules(&self) -> Vec<CompiledRule> {
        self.rules.try_read().map(|r| r.clone()).unwrap_or_default()
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("test_mode", &self.test_mode)
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
