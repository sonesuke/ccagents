use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use crate::agent::Agent;
use crate::config::rules_config::{Rule, RuleType};
use crate::rule::capture::resolve_capture_groups_in_vec;
use crate::config::helper::ActionType;

/// When condition processor for PTY output pattern matching
pub struct When {
    rules: Arc<RwLock<Vec<Rule>>>,
    agent: Arc<Agent>,
}

impl When {
    pub fn new(rules: Vec<Rule>, agent: Arc<Agent>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(rules)),
            agent,
        }
    }

    /// Process rules for the given PTY output
    pub async fn process_rules_for_output(&self, pty_output: &str) -> Result<()> {
        // Remove ANSI escape sequences for cleaner pattern matching
        let clean_output = self.strip_ansi_escapes(pty_output);

        tracing::debug!("=== PTY OUTPUT ===");
        tracing::debug!("Raw output: {:?}", pty_output);
        tracing::debug!("Clean output: {:?}", clean_output);
        tracing::debug!("==> Will check rules for PTY output");

        // Split by both \n and \r for better handling of carriage returns
        let lines: Vec<&str> = clean_output
            .split(['\n', '\r'])
            .filter(|line| !line.trim().is_empty())
            .collect();

        // Check each line for pattern matching
        for line in lines {
            tracing::debug!("Checking line: {:?}", line);

            let rules = self.rules.read().await;
            let action = decide_action(line, &rules);

            if !matches!(action, ActionType::SendKeys(ref keys) if keys.is_empty()) {
                tracing::debug!("Action decided: {:?}", action);
                execute_rule_action(&action, &self.agent, "Rule action").await?;
            }
        }

        Ok(())
    }

    /// Start monitoring PTY output for rule processing
    pub async fn start_monitoring(&self, mut receiver: broadcast::Receiver<String>) -> Result<()> {
        loop {
            self.receive_and_process_pty_output(&mut receiver).await?;

            // Small delay to prevent busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    async fn receive_and_process_pty_output(
        &self,
        receiver: &mut broadcast::Receiver<String>,
    ) -> Result<()> {
        let mut received_any = false;

        while let Ok(pty_output) = receiver.try_recv() {
            received_any = true;
            tracing::debug!(
                "📝 Agent {} received PTY output: {} bytes: '{}'",
                self.agent.get_id(),
                pty_output.len(),
                pty_output.chars().take(50).collect::<String>()
            );

            // Process rules only for Active agents
            if self.agent.is_active().await {
                tracing::debug!("🔍 Processing rules for agent {}", self.agent.get_id());
                if let Err(e) = self.process_rules_for_output(&pty_output).await {
                    tracing::debug!("❌ Error processing PTY output: {}", e);
                }
            } else {
                tracing::trace!(
                    "⏸️  Skipping rule processing for agent {} (inactive)",
                    self.agent.get_id()
                );
            }
        }

        if received_any {
            tracing::debug!("✅ Agent {} processed data chunks", self.agent.get_id());
        }

        Ok(())
    }

    /// Strip ANSI escape sequences from text
    fn strip_ansi_escapes(&self, text: &str) -> String {
        let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap();
        ansi_regex.replace_all(text, "").to_string()
    }
}

/// Execute an action for rules (not for triggers)
async fn execute_rule_action(
    action: &crate::config::helper::ActionType,
    agent: &Agent,
    context: &str,
) -> Result<()> {
    let crate::config::helper::ActionType::SendKeys(keys) = action;
    if keys.is_empty() {
        tracing::debug!("{}: No keys to send", context);
        return Ok(());
    }

    tracing::info!("{}: Sending {} keys", context, keys.len());
    tracing::debug!("{}: Keys: {:?}", context, keys);

    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        agent.send_keys(key).await?;
    }

    Ok(())
}

/// Matches capture text against parsed rules and returns the appropriate action.
///
/// This function iterates through rules in priority order (as loaded by load_config)
/// and returns the first matching rule's action with resolved capture groups.
/// If no rules match, returns ActionType::SendKeys(vec![]) as the default.
///
/// # Arguments
/// * `capture` - The text to match against rule patterns
/// * `rules` - Slice of parsed rules, assumed to be sorted by priority
///
/// # Returns
/// An ActionType representing the action to take
///
/// # Performance
/// Early termination on first match ensures optimal performance.
/// Should complete within 1ms for 100 rules with typical patterns.
pub fn decide_action(capture: &str, rules: &[Rule]) -> ActionType {
    // Debug log the capture being checked
    if !capture.trim().is_empty() {
        tracing::debug!("🔍 Checking capture against rules: {:?}", capture);
    }

    // Log to file for debugging
    tracing::debug!("CHECKING: {:?}", capture);
    tracing::debug!("  Clean content: {}", capture.trim());
    tracing::debug!("  Length: {}", capture.len());
    tracing::debug!("  Rules to check: {}", rules.len());

    for (i, rule) in rules.iter().enumerate() {
        match &rule.rule_type {
            RuleType::When(regex) => {
                if let Some(captures) = regex.captures(capture) {
                    // Log the matched pattern
                    tracing::info!(
                        "✅ Pattern matched! Pattern: {:?}, Capture: {:?}",
                        regex.as_str(),
                        capture
                    );

                    // Log matched rule details
                    tracing::debug!("  ✅ MATCHED! Rule #{} Pattern: {:?}", i, regex.as_str());
                    tracing::debug!("     Action: {:?}", rule.action);
                    tracing::debug!("     Full match: {:?}", captures.get(0).map(|m| m.as_str()));
                    tracing::debug!("---");

                    // Extract capture groups
                    let captured_groups: Vec<String> = captures
                        .iter()
                        .skip(1) // Skip the full match (index 0)
                        .filter_map(|m| m.map(|m| m.as_str().to_string()))
                        .collect();

                    // Resolve capture groups in the action
                    let resolved_action = match &rule.action {
                        ActionType::SendKeys(keys) => ActionType::SendKeys(
                            resolve_capture_groups_in_vec(keys, &captured_groups),
                        ),
                    };

                    return resolved_action;
                }
            }
            RuleType::DiffTimeout(_) => {
                // Timeout rules are handled by diff_timeout module
                continue;
            }
        }
    }

    // Default case: no rules matched
    ActionType::SendKeys(vec![])
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

    #[test]
    fn test_decide_action_exact_match() {
        let rules = vec![
            create_test_rule(
                r"issue\s+(\d+)",
                vec!["open_issue".to_string(), "${1}".to_string()],
            ),
            create_test_rule(r"resume", vec!["resume_task".to_string()]),
        ];

        let action = decide_action("issue 123", &rules);
        assert_eq!(
            action,
            ActionType::SendKeys(vec!["open_issue".to_string(), "123".to_string()])
        );
    }

    #[test]
    fn test_decide_action_priority_ordering() {
        let rules = vec![
            create_test_rule(r"test", vec!["high_priority".to_string()]),
            create_test_rule(r"test", vec!["low_priority".to_string()]),
        ];

        // Should match the first rule (higher priority - lower number)
        let action = decide_action("test", &rules);
        assert_eq!(
            action,
            ActionType::SendKeys(vec!["high_priority".to_string()])
        );
    }

    #[test]
    fn test_decide_action_no_match() {
        let rules = vec![
            create_test_rule(r"issue\s+(\d+)", vec!["open_issue".to_string()]),
            create_test_rule(r"resume", vec!["resume_task".to_string()]),
        ];

        let action = decide_action("no matching pattern here", &rules);
        assert_eq!(action, ActionType::SendKeys(vec![]));
    }

    #[test]
    fn test_decide_action_empty_capture() {
        let rules = vec![create_test_rule(
            r"issue\s+(\d+)",
            vec!["open_issue".to_string()],
        )];

        let action = decide_action("", &rules);
        assert_eq!(action, ActionType::SendKeys(vec![]));
    }

    #[test]
    fn test_decide_action_empty_rules() {
        let action = decide_action("any text", &[]);
        assert_eq!(action, ActionType::SendKeys(vec![]));
    }

    #[test]
    fn test_decide_action_capture_groups() {
        let rules = vec![create_test_rule(
            r"deploy\s+(\w+)\s+to\s+(\w+)",
            vec!["deploy".to_string(), "${1}".to_string(), "${2}".to_string()],
        )];

        let action = decide_action("deploy app to production", &rules);
        assert_eq!(
            action,
            ActionType::SendKeys(vec![
                "deploy".to_string(),
                "app".to_string(),
                "production".to_string()
            ])
        );
    }

    #[test]
    fn test_performance_100_rules() {
        use std::time::Instant;

        // Create 100 test rules that don't match our test input
        let rules: Vec<Rule> = (0..100)
            .map(|i| create_test_rule(&format!("unique_pattern_{}", i), vec![]))
            .collect();

        let start = Instant::now();
        let action = decide_action("non-matching test input", &rules);
        let duration = start.elapsed();

        assert_eq!(action, ActionType::SendKeys(vec![]));
        assert!(
            duration.as_millis() < 100,
            "Should complete within 100ms for 100 rules, took {}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_japanese_hello_pattern() {
        let rules = vec![create_test_rule(
            r"こんにちは|Hello",
            vec!["q".to_string(), "\r".to_string()],
        )];

        // Test case from actual log
        let actual_content = "Users/sonesuke/rule-agents                │\n╰───────────────────────────────────────────────────╯\n\n\n> say hello, in Japanese\n\n⏺ こんにちは！\n\n╭──────────────────────────────────────────────────────────────────────────────╮\n│ > Try \"how does compiled_rule.rs work?\"                                      │\n╰──────────────────────────────────────────────────────────────────────────────╯\n  ? for shortcuts";

        let action = decide_action(actual_content, &rules);

        // This should match!
        assert_eq!(
            action,
            ActionType::SendKeys(vec!["q".to_string(), "\r".to_string()]),
            "Pattern should match こんにちは in the content"
        );
    }
}
