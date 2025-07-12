use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::agent::Agent;
use crate::config::rule::{CompiledRule, RuleType};
use crate::config::types::ActionType;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration as TokioDuration;

use super::Monitor;

/// Diff timeout processor responsible for checking diff_timeout rules across all agents
pub struct DiffTimeout {
    rules: Arc<RwLock<Vec<CompiledRule>>>,
    timeout_state: Arc<Mutex<TimeoutState>>,
    pub agents: Arc<Vec<Arc<Agent>>>,
}

impl Monitor for DiffTimeout {
    async fn start_monitoring(self) -> Result<()> {
        self.start_monitoring().await
    }
}

impl DiffTimeout {
    pub fn new(rules: Vec<CompiledRule>, agents: Arc<Vec<Arc<Agent>>>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(rules)),
            timeout_state: Arc::new(Mutex::new(TimeoutState::new())),
            agents,
        }
    }

    /// Reset timeout activity (called when terminal output is received)
    pub async fn reset_timeout_activity(&self) {
        let mut timeout_state = self.timeout_state.lock().await;
        timeout_state.reset_activity();
    }

    pub async fn start_monitoring(self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            // Check timeout rules for active agents only
            self.check_timeout_rules().await?;
        }
    }

    async fn check_timeout_rules(&self) -> Result<()> {
        // Only process timeout rules when there are active agents
        for agent in self.agents.iter() {
            if agent.is_active().await {
                let rules = self.rules.read().await;
                let mut timeout_state = self.timeout_state.lock().await;
                let timeout_actions = check_timeout_rules(&rules, &mut timeout_state);

                for action in timeout_actions {
                    tracing::info!("⏰ Executing timeout rule action: {:?}", action);
                    if let Err(e) = execute_rule_action(&action, agent, "🤖 Rule action").await {
                        tracing::error!("❌ Error executing timeout rule action: {}", e);
                    }
                }
                break; // Only need to check once per cycle if any agent is active
            }
        }

        Ok(())
    }
}

/// Timeout state tracker for diff timeout rules
#[derive(Debug)]
pub struct TimeoutState {
    last_activity: Instant,
    timeout_timers: Vec<(Duration, bool)>, // (duration, triggered)
}

impl TimeoutState {
    pub fn new() -> Self {
        Self {
            last_activity: Instant::now(),
            timeout_timers: Vec::new(),
        }
    }

    pub fn reset_activity(&mut self) {
        self.last_activity = Instant::now();
        // Reset all timeout triggers
        for (_, triggered) in &mut self.timeout_timers {
            *triggered = false;
        }
    }

    pub fn check_timeouts(&mut self, timeout_durations: &[Duration]) -> Vec<usize> {
        let elapsed = self.last_activity.elapsed();
        let mut triggered_indices = Vec::new();

        // Initialize timers if needed
        if self.timeout_timers.len() != timeout_durations.len() {
            self.timeout_timers = timeout_durations.iter().map(|&d| (d, false)).collect();
        }

        // Check each timeout
        for (i, (duration, triggered)) in self.timeout_timers.iter_mut().enumerate() {
            if elapsed >= *duration && !*triggered {
                *triggered = true;
                triggered_indices.push(i);
            }
        }

        triggered_indices
    }
}

/// Check timeout rules and return triggered actions
pub fn check_timeout_rules(
    rules: &[CompiledRule],
    timeout_state: &mut TimeoutState,
) -> Vec<ActionType> {
    let mut triggered_actions = Vec::new();

    // Extract timeout durations from rules
    let timeout_durations: Vec<Duration> = rules
        .iter()
        .filter_map(|rule| match &rule.rule_type {
            RuleType::DiffTimeout(duration) => Some(*duration),
            _ => None,
        })
        .collect();

    // Check for triggered timeouts
    let triggered_indices = timeout_state.check_timeouts(&timeout_durations);

    // Find corresponding actions for triggered timeouts
    let mut timeout_rule_index = 0;
    for (rule_index, rule) in rules.iter().enumerate() {
        if let RuleType::DiffTimeout(_) = &rule.rule_type {
            if triggered_indices.contains(&timeout_rule_index) {
                tracing::info!(
                    "⏰ Timeout triggered! Rule #{} Duration: {:?}",
                    rule_index,
                    match &rule.rule_type {
                        RuleType::DiffTimeout(d) => d,
                        _ => unreachable!(),
                    }
                );
                triggered_actions.push(rule.action.clone());
            }
            timeout_rule_index += 1;
        }
    }

    triggered_actions
}

/// Execute an action for rules (not for triggers)
async fn execute_rule_action(action: &ActionType, agent: &Agent, context: &str) -> Result<()> {
    let ActionType::SendKeys(keys) = action;
    if keys.is_empty() {
        tracing::debug!("{}: No keys to send", context);
        return Ok(());
    }

    tracing::info!("{}: Sending {} keys", context, keys.len());
    tracing::debug!("{}: Keys: {:?}", context, keys);

    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(TokioDuration::from_millis(100)).await;
        }
        agent.send_keys(key).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rule::RuleType;

    fn create_timeout_rule(duration_str: &str, keys: Vec<String>) -> CompiledRule {
        let duration = match duration_str.strip_suffix('s') {
            Some(n) => Duration::from_secs(n.parse().unwrap()),
            None => match duration_str.strip_suffix('m') {
                Some(n) => Duration::from_secs(n.parse::<u64>().unwrap() * 60),
                None => Duration::from_secs(1), // fallback
            },
        };
        CompiledRule {
            rule_type: RuleType::DiffTimeout(duration),
            action: ActionType::SendKeys(keys),
        }
    }

    #[test]
    fn test_timeout_state_new() {
        let state = TimeoutState::new();
        assert!(state.last_activity.elapsed() < Duration::from_millis(100));
        assert_eq!(state.timeout_timers.len(), 0);
    }

    #[test]
    fn test_timeout_state_reset_activity() {
        let mut state = TimeoutState::new();
        std::thread::sleep(Duration::from_millis(10));
        state.reset_activity();
        assert!(state.last_activity.elapsed() < Duration::from_millis(10));
    }

    #[test]
    fn test_check_timeout_rules() {
        let rules = vec![
            create_timeout_rule("1s", vec!["timeout1".to_string()]),
            create_timeout_rule("2s", vec!["timeout2".to_string()]),
        ];

        let mut timeout_state = TimeoutState::new();
        // Set last activity to 1.5 seconds ago
        timeout_state.last_activity = Instant::now() - Duration::from_millis(1500);

        let actions = check_timeout_rules(&rules, &mut timeout_state);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout1".to_string()])
        );
    }

    #[test]
    fn test_multiple_timeout_rules() {
        let rules = vec![
            create_timeout_rule("1s", vec!["short_timeout".to_string()]),
            create_timeout_rule("2s", vec!["long_timeout".to_string()]),
        ];

        let mut timeout_state = TimeoutState::new();
        // Set last activity to 2.5 seconds ago
        timeout_state.last_activity = Instant::now() - Duration::from_millis(2500);

        let actions = check_timeout_rules(&rules, &mut timeout_state);
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

    #[test]
    fn test_diff_timeout_multiple_triggers() {
        let rules = vec![create_timeout_rule(
            "1s",
            vec!["timeout_action".to_string()],
        )];

        let mut timeout_state = TimeoutState::new();

        // First timeout trigger
        timeout_state.last_activity = Instant::now() - Duration::from_millis(1500);
        let actions = check_timeout_rules(&rules, &mut timeout_state);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout_action".to_string()])
        );

        // Reset activity (simulating terminal output)
        timeout_state.reset_activity();

        // Should not trigger immediately after reset
        let actions = check_timeout_rules(&rules, &mut timeout_state);
        assert_eq!(actions.len(), 0);

        // Second timeout trigger after reset
        timeout_state.last_activity = Instant::now() - Duration::from_millis(1500);
        let actions = check_timeout_rules(&rules, &mut timeout_state);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout_action".to_string()])
        );
    }
}
