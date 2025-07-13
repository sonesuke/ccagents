use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::agent::Agent;
use crate::config::helper::ActionType;
use crate::config::rules_config::{Rule, RuleType};
use tokio::sync::broadcast;
use tokio::time::interval;

use super::{RuleProcessor, execute_rule_action};

/// Diff timeout processor responsible for checking diff_timeout rules for a single agent
pub struct DiffTimeout {
    durations: Vec<Duration>,
    pub(crate) actions: Vec<ActionType>,
    agent: Arc<Agent>,
    last_activity: std::sync::Mutex<Instant>,
    timeout_timers: std::sync::Mutex<Vec<TimeoutTimer>>,
}

/// Configuration for monitoring intervals
const MONITORING_INTERVAL_MS: u64 = 100;

impl RuleProcessor for DiffTimeout {
    async fn start_monitoring(&self, mut receiver: broadcast::Receiver<String>) -> Result<()> {
        let mut check_interval = interval(Duration::from_millis(MONITORING_INTERVAL_MS));

        loop {
            tokio::select! {
                // Listen for PTY output to reset timeout
                result = receiver.recv() => {
                    match result {
                        Ok(_) => {
                            self.reset_timeout_activity().await;
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!("DiffTimeout receiver lagged, skipped {} messages", skipped);
                            self.reset_timeout_activity().await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("DiffTimeout receiver closed, stopping monitoring");
                            break;
                        }
                    }
                }
                // Check timeout rules periodically
                _ = check_interval.tick() => {
                    if let Err(e) = self.process_timeout_rules().await {
                        tracing::error!("Error checking timeout rules: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

impl DiffTimeout {
    pub fn new(rules: Vec<Rule>, agent: Arc<Agent>) -> Self {
        // Filter to only keep DiffTimeout rules and extract durations and actions
        let diff_timeout_rules: Vec<Rule> = rules
            .into_iter()
            .filter(|rule| matches!(rule.rule_type, RuleType::DiffTimeout(_)))
            .collect();

        let durations: Vec<Duration> = diff_timeout_rules
            .iter()
            .map(|rule| {
                if let RuleType::DiffTimeout(duration) = &rule.rule_type {
                    *duration
                } else {
                    panic!("Only DiffTimeout rules should be present")
                }
            })
            .collect();

        let actions: Vec<ActionType> = diff_timeout_rules
            .into_iter()
            .map(|rule| rule.action)
            .collect();

        let timers = durations
            .iter()
            .map(|&duration| TimeoutTimer {
                duration,
                triggered: false,
            })
            .collect();

        Self {
            durations,
            actions,
            agent,
            last_activity: std::sync::Mutex::new(Instant::now()),
            timeout_timers: std::sync::Mutex::new(timers),
        }
    }

    /// Reset timeout activity (called when terminal output is received)
    async fn reset_timeout_activity(&self) {
        if let (Ok(mut last_activity), Ok(mut timers)) =
            (self.last_activity.lock(), self.timeout_timers.lock())
        {
            *last_activity = Instant::now();
            for timer in timers.iter_mut() {
                timer.triggered = false;
            }
        }
    }

    async fn process_timeout_rules(&self) -> Result<()> {
        if !self.agent.is_active().await {
            return Ok(());
        }

        let triggered_indices = self.find_triggered_timeout_indices();

        for idx in triggered_indices {
            let duration = self.durations[idx];
            let action = &self.actions[idx];

            tracing::info!(
                "⏰ Timeout triggered! Rule #{} Duration: {:?}",
                idx,
                duration
            );
            tracing::info!("⏰ Executing timeout rule action: {:?}", action);

            if let Err(e) = execute_rule_action(action, &self.agent, "🤖 Rule action").await {
                tracing::error!("❌ Error executing timeout rule action: {}", e);
            }
        }

        Ok(())
    }

    pub(crate) fn find_triggered_timeout_indices(&self) -> Vec<usize> {
        let Ok(last_activity) = self.last_activity.lock() else {
            return Vec::new();
        };
        let Ok(mut timers) = self.timeout_timers.lock() else {
            return Vec::new();
        };

        let elapsed = last_activity.elapsed();
        drop(last_activity);

        timers
            .iter_mut()
            .enumerate()
            .filter_map(|(i, timer)| {
                if elapsed >= timer.duration && !timer.triggered {
                    timer.triggered = true;
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Individual timeout timer state
#[derive(Debug, Clone)]
struct TimeoutTimer {
    duration: Duration,
    triggered: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rules_config::RuleType;

    fn create_timeout_rule(duration_str: &str, keys: Vec<String>) -> Rule {
        Rule {
            rule_type: RuleType::DiffTimeout(parse_duration(duration_str)),
            action: ActionType::SendKeys(keys),
        }
    }

    fn parse_duration(duration_str: &str) -> Duration {
        if let Some(seconds_str) = duration_str.strip_suffix('s') {
            Duration::from_secs(seconds_str.parse().unwrap_or(1))
        } else if let Some(minutes_str) = duration_str.strip_suffix('m') {
            Duration::from_secs(minutes_str.parse::<u64>().unwrap_or(1) * 60)
        } else {
            Duration::from_secs(1)
        }
    }

    #[tokio::test]
    async fn test_check_timeout_rules() {
        use crate::agent::Agent;
        use crate::config::Config;
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false;
        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        let rules = vec![
            create_timeout_rule("1s", vec!["timeout1".to_string()]),
            create_timeout_rule("2s", vec!["timeout2".to_string()]),
        ];

        let diff_timeout = DiffTimeout::new(rules.clone(), agent);

        // Simulate 1.5 seconds elapsed
        if let Ok(mut last_activity) = diff_timeout.last_activity.lock() {
            *last_activity = Instant::now() - Duration::from_millis(1500);
        }

        let indices = diff_timeout.find_triggered_timeout_indices();
        let actions: Vec<ActionType> = indices
            .into_iter()
            .map(|i| diff_timeout.actions[i].clone())
            .collect();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout1".to_string()])
        );
    }

    #[tokio::test]
    async fn test_multiple_timeout_rules() {
        use crate::agent::Agent;
        use crate::config::Config;
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false;
        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        let rules = vec![
            create_timeout_rule("1s", vec!["short_timeout".to_string()]),
            create_timeout_rule("2s", vec!["long_timeout".to_string()]),
        ];

        let diff_timeout = DiffTimeout::new(rules.clone(), agent);

        // Simulate 2.5 seconds elapsed
        if let Ok(mut last_activity) = diff_timeout.last_activity.lock() {
            *last_activity = Instant::now() - Duration::from_millis(2500);
        }

        let indices = diff_timeout.find_triggered_timeout_indices();
        let actions: Vec<ActionType> = indices
            .into_iter()
            .map(|i| diff_timeout.actions[i].clone())
            .collect();
        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["short_timeout".to_string()])
        );
        assert_eq!(
            actions[1],
            ActionType::SendKeys(vec!["long_timeout".to_string()])
        );
    }

    #[tokio::test]
    async fn test_diff_timeout_multiple_triggers() {
        use crate::agent::Agent;
        use crate::config::Config;
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false;
        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        let rules = vec![create_timeout_rule(
            "1s",
            vec!["timeout_action".to_string()],
        )];

        let diff_timeout = DiffTimeout::new(rules.clone(), agent);

        // First timeout trigger
        if let Ok(mut last_activity) = diff_timeout.last_activity.lock() {
            *last_activity = Instant::now() - Duration::from_millis(1500);
        }
        let indices = diff_timeout.find_triggered_timeout_indices();
        let actions: Vec<ActionType> = indices
            .into_iter()
            .map(|i| diff_timeout.actions[i].clone())
            .collect();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout_action".to_string()])
        );

        // Reset activity (simulating terminal output)
        diff_timeout.reset_timeout_activity().await;

        // Should not trigger immediately after reset
        let indices = diff_timeout.find_triggered_timeout_indices();
        let actions: Vec<ActionType> = indices
            .into_iter()
            .map(|i| diff_timeout.actions[i].clone())
            .collect();
        assert_eq!(actions.len(), 0);

        // Second timeout trigger after reset
        if let Ok(mut last_activity) = diff_timeout.last_activity.lock() {
            *last_activity = Instant::now() - Duration::from_millis(1500);
        }
        let indices = diff_timeout.find_triggered_timeout_indices();
        let actions: Vec<ActionType> = indices
            .into_iter()
            .map(|i| diff_timeout.actions[i].clone())
            .collect();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout_action".to_string()])
        );
    }
}
