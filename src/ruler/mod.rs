pub mod config;
pub mod decision;
pub mod entry;
pub mod rule;
pub mod types;

use crate::ruler::config::load_config;
use crate::ruler::decision::{TimeoutState, decide_action_with_timeout};
use crate::ruler::entry::{CompiledEntry, TriggerType};
use crate::ruler::rule::CompiledRule;
use crate::ruler::types::ActionType;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct Ruler {
    entries: Arc<RwLock<Vec<CompiledEntry>>>,
    rules: Arc<RwLock<Vec<CompiledRule>>>,
    timeout_state: Arc<Mutex<TimeoutState>>,
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
            timeout_state: Arc::new(Mutex::new(TimeoutState::new())),
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

    /// Enhanced decision function that handles both pattern matching and timeout rules
    pub async fn decide_actions_with_timeout(&self, capture: &str) -> Vec<ActionType> {
        let rules = self.get_rules().await;
        let mut timeout_state = self.timeout_state.lock().await;
        decide_action_with_timeout(capture, &rules, &mut timeout_state)
    }

    /// Check only timeout rules (useful for periodic checks)
    pub async fn check_timeout_rules(&self) -> Vec<ActionType> {
        let rules = self.get_rules().await;
        let mut timeout_state = self.timeout_state.lock().await;
        crate::ruler::decision::check_timeout_rules(&rules, &mut timeout_state)
    }

    /// Reset timeout activity (called when any terminal output is received)
    pub async fn reset_timeout_activity(&self) {
        let mut timeout_state = self.timeout_state.lock().await;
        timeout_state.reset_activity();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_reset_timeout_activity() {
        // Use existing diff-timeout-demo config
        let config_path = "examples/diff-timeout-demo/config.yaml";
        let ruler = Ruler::new(config_path).await.unwrap();

        // Manually set last activity to simulate timeout condition
        {
            let mut timeout_state = ruler.timeout_state.lock().await;
            timeout_state.set_last_activity_for_test(
                std::time::Instant::now() - Duration::from_millis(31000), // 31 seconds ago to trigger 30s timeout
            );
        }

        // Should have timeout action available
        let actions = ruler.check_timeout_rules().await;
        assert!(
            !actions.is_empty(),
            "Should have timeout actions when 31 seconds have passed"
        );

        // Reset timeout activity
        ruler.reset_timeout_activity().await;

        // Should no longer have timeout actions
        let actions = ruler.check_timeout_rules().await;
        assert!(
            actions.is_empty(),
            "Should not have timeout actions after reset"
        );
    }
}
