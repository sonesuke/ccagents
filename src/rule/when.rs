use anyhow::Result;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::agent::Agent;
use crate::config::helper::ActionType;
use crate::config::rules_config::{Rule, RuleType};
use crate::rule::{RuleProcessor, execute_rule_action};

/// Delay between PTY output checks to prevent busy waiting
const MONITORING_INTERVAL_MS: u64 = 10;

/// When condition processor for PTY output pattern matching
pub struct When {
    regexes: Vec<Regex>,
    actions: Vec<ActionType>,
    agent: Arc<Agent>,
}

impl RuleProcessor for When {
    async fn start_monitoring(&self, mut receiver: broadcast::Receiver<String>) -> Result<()> {
        loop {
            while let Ok(pty_output) = receiver.try_recv() {
                if self.agent.is_active().await {
                    let lines = self.get_normalized_lines(&pty_output);

                    // Check each line for pattern matching
                    for line in lines {
                        let action = self.decide_action(&line);

                        if !matches!(action, ActionType::SendKeys(ref keys) if keys.is_empty()) {
                            if let Err(e) =
                                execute_rule_action(&action, &self.agent, "Rule action").await
                            {
                                tracing::error!("Error executing rule action: {}", e);
                            }
                        }
                    }
                }
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(MONITORING_INTERVAL_MS)).await;
        }
    }
}

impl When {
    pub fn new(rules: Vec<Rule>, agent: Arc<Agent>) -> Self {
        // Filter to only keep When rules and extract regexes and actions
        let when_rules: Vec<Rule> = rules
            .into_iter()
            .filter(|rule| matches!(rule.rule_type, RuleType::When(_)))
            .collect();

        let regexes: Vec<Regex> = when_rules
            .iter()
            .map(|rule| {
                if let RuleType::When(regex) = &rule.rule_type {
                    regex.clone()
                } else {
                    panic!("Only When rules should be present")
                }
            })
            .collect();

        let actions: Vec<ActionType> = when_rules.into_iter().map(|rule| rule.action).collect();

        Self {
            regexes,
            actions,
            agent,
        }
    }

    /// Decides what action to take based on a terminal output capture
    fn decide_action(&self, capture: &str) -> ActionType {
        for (i, regex) in self.regexes.iter().enumerate() {
            if let Some(captures) = regex.captures(capture) {
                let mut action = self.actions[i].clone();

                // Handle capture group substitution
                let ActionType::SendKeys(ref mut keys) = action;
                for key in keys.iter_mut() {
                    // Replace ${1}, ${2}, etc. with capture groups
                    for (j, capture_group) in captures.iter().enumerate().skip(1) {
                        if let Some(matched) = capture_group {
                            *key = key.replace(&format!("${{{}}}", j), matched.as_str());
                        }
                    }
                }

                return action;
            }
        }

        // Return empty action if no rule matches
        ActionType::SendKeys(vec![])
    }

    /// Normalize PTY output into clean lines for pattern matching
    fn get_normalized_lines(&self, pty_output: &str) -> Vec<String> {
        // Remove ANSI escape sequences for cleaner pattern matching
        let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap();
        let clean_output = ansi_regex.replace_all(pty_output, "");

        // Split by both \n and \r for better handling of carriage returns
        clean_output
            .split(['\n', '\r'])
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rules_config::RuleType;
    use regex::Regex;

    fn create_test_rule(pattern: &str, keys: Vec<String>) -> Rule {
        Rule {
            rule_type: RuleType::When(Regex::new(pattern).unwrap()),
            action: ActionType::SendKeys(keys),
        }
    }

    async fn create_test_agent() -> Arc<Agent> {
        use crate::agent::Agent;
        use crate::config::Config;
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false;
        let mock_pty = Box::new(MockPtyProcess::new());
        Agent::new_with_process(0, &config, mock_pty).await.unwrap()
    }

    #[tokio::test]
    async fn test_decide_action_exact_match() {
        let rules = vec![
            create_test_rule(
                r"issue\s+(\d+)",
                vec!["open_issue".to_string(), "${1}".to_string()],
            ),
            create_test_rule(r"resume", vec!["resume_task".to_string()]),
        ];

        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action("issue 123");
        assert_eq!(
            action,
            ActionType::SendKeys(vec!["open_issue".to_string(), "123".to_string()])
        );
    }

    #[tokio::test]
    async fn test_decide_action_priority_ordering() {
        let rules = vec![
            create_test_rule(r"test", vec!["high_priority".to_string()]),
            create_test_rule(r"test", vec!["low_priority".to_string()]),
        ];

        // Should match the first rule (higher priority - lower number)
        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action("test");
        assert_eq!(
            action,
            ActionType::SendKeys(vec!["high_priority".to_string()])
        );
    }

    #[tokio::test]
    async fn test_decide_action_no_match() {
        let rules = vec![
            create_test_rule(r"issue\s+(\d+)", vec!["open_issue".to_string()]),
            create_test_rule(r"resume", vec!["resume_task".to_string()]),
        ];

        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action("no matching pattern here");
        assert_eq!(action, ActionType::SendKeys(vec![]));
    }

    #[tokio::test]
    async fn test_decide_action_empty_capture() {
        let rules = vec![create_test_rule(
            r"issue\s+(\d+)",
            vec!["open_issue".to_string()],
        )];

        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action("");
        assert_eq!(action, ActionType::SendKeys(vec![]));
    }

    #[tokio::test]
    async fn test_decide_action_empty_rules() {
        let when = When::new(vec![], create_test_agent().await);
        let action = when.decide_action("any text");
        assert_eq!(action, ActionType::SendKeys(vec![]));
    }

    #[tokio::test]
    async fn test_decide_action_capture_groups() {
        let rules = vec![create_test_rule(
            r"deploy\s+(\w+)\s+to\s+(\w+)",
            vec!["deploy".to_string(), "${1}".to_string(), "${2}".to_string()],
        )];

        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action("deploy app to production");
        assert_eq!(
            action,
            ActionType::SendKeys(vec![
                "deploy".to_string(),
                "app".to_string(),
                "production".to_string()
            ])
        );
    }

    #[tokio::test]
    async fn test_performance_100_rules() {
        use std::time::Instant;

        // Create 100 test rules that don't match our test input
        let rules: Vec<Rule> = (0..100)
            .map(|i| create_test_rule(&format!("unique_pattern_{}", i), vec![]))
            .collect();

        let start = Instant::now();
        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action("non-matching test input");
        let duration = start.elapsed();

        assert_eq!(action, ActionType::SendKeys(vec![]));
        assert!(
            duration.as_millis() < 200,
            "Should complete within 200ms for 100 rules, took {}ms",
            duration.as_millis()
        );
    }

    #[tokio::test]
    async fn test_japanese_hello_pattern() {
        let rules = vec![create_test_rule(
            r"こんにちは|Hello",
            vec!["q".to_string(), "\r".to_string()],
        )];

        // Test case from actual log
        let actual_content = "Users/sonesuke/rule-agents                │\n╰───────────────────────────────────────────────────╯\n\n\n> say hello, in Japanese\n\n⏺ こんにちは！\n\n╭──────────────────────────────────────────────────────────────────────────────╮\n│ > Try \"how does compiled_rule.rs work?\"                                      │\n╰──────────────────────────────────────────────────────────────────────────────╯\n  ? for shortcuts";

        let when = When::new(rules, create_test_agent().await);
        let action = when.decide_action(actual_content);

        // This should match!
        assert_eq!(
            action,
            ActionType::SendKeys(vec!["q".to_string(), "\r".to_string()]),
            "Pattern should match こんにちは in the content"
        );
    }
}
