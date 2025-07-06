pub mod config;
pub mod decision;
pub mod entry;
pub mod rule;
pub mod types;

use crate::ruler::config::load_config;
use crate::ruler::decision::decide_action;
use crate::ruler::entry::{CompiledEntry, TriggerType};
use crate::ruler::rule::CompiledRule;
use crate::ruler::types::ActionType;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Ruler {
    entries: Arc<RwLock<Vec<CompiledEntry>>>,
    rules: Arc<RwLock<Vec<CompiledRule>>>,
    test_mode: bool,
    // Monitor configuration
    monitor_config: config::MonitorConfig,
}

impl Ruler {
    pub async fn new(config_path: &str) -> Result<Self> {
        // Load initial configuration (entries, rules, and monitor config)
        let (initial_entries, initial_rules, monitor_config) =
            load_config(std::path::Path::new(config_path))?;
        let entries = Arc::new(RwLock::new(initial_entries.clone()));
        let rules = Arc::new(RwLock::new(initial_rules));

        // In test environment, create a simple mock backend that always succeeds
        let is_test = std::env::var("CARGO_TEST").is_ok()
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
                .unwrap_or(false);

        Ok(Ruler {
            entries,
            rules,
            test_mode: is_test,
            monitor_config,
        })
    }

    pub async fn get_entries(&self) -> Vec<CompiledEntry> {
        self.entries.read().await.clone()
    }

    pub async fn get_rules(&self) -> Vec<CompiledRule> {
        self.rules.read().await.clone()
    }

    pub async fn get_on_start_entries(&self) -> Vec<CompiledEntry> {
        let entries = self.get_entries().await;
        entries
            .into_iter()
            .filter(|entry| matches!(entry.trigger, TriggerType::OnStart))
            .collect()
    }

    pub async fn get_periodic_entries(&self) -> Vec<CompiledEntry> {
        let entries = self.get_entries().await;
        entries
            .into_iter()
            .filter(|entry| matches!(entry.trigger, TriggerType::Periodic { .. }))
            .collect()
    }

    pub async fn decide_action_for_capture(&self, capture: &str) -> ActionType {
        let rules = self.get_rules().await;
        decide_action(capture, &rules)
    }

    /// Get monitor configuration
    pub fn get_monitor_config(&self) -> &config::MonitorConfig {
        &self.monitor_config
    }
}

impl std::fmt::Debug for Ruler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ruler")
            .field("test_mode", &self.test_mode)
            .finish()
    }
}
