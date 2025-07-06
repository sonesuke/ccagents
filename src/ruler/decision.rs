use crate::ruler::rule::resolve_capture_groups_in_vec;
use crate::ruler::rule::{CompiledRule, RuleType};
use crate::ruler::types::ActionType;
use std::time::{Duration, Instant};

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

/// Matches capture text against compiled rules and returns the appropriate action.
///
/// This function iterates through rules in priority order (as loaded by load_config)
/// and returns the first matching rule's action with resolved capture groups.
/// If no rules match, returns ActionType::SendKeys(vec![]) as the default.
///
/// # Arguments
/// * `capture` - The text to match against rule patterns
/// * `rules` - Slice of compiled rules, assumed to be sorted by priority
///
/// # Returns
/// An ActionType representing the action to take
///
/// # Performance
/// Early termination on first match ensures optimal performance.
/// Should complete within 1ms for 100 rules with typical patterns.
pub fn decide_action(capture: &str, rules: &[CompiledRule]) -> ActionType {
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
            RuleType::Pattern(regex) => {
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
                        ActionType::Workflow(workflow, args) => {
                            let resolved_args =
                                resolve_capture_groups_in_vec(args, &captured_groups);
                            ActionType::Workflow(workflow.clone(), resolved_args)
                        }
                    };

                    return resolved_action;
                }
            }
            RuleType::DiffTimeout(_) => {
                // Timeout rules are handled by a separate function
                // This function only handles pattern matching
                continue;
            }
        }
    }

    // Default case: no rules matched
    ActionType::SendKeys(vec![])
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

/// Combined decision function that handles both pattern matching and timeout rules
pub fn decide_action_with_timeout(
    capture: &str,
    rules: &[CompiledRule],
    timeout_state: &mut TimeoutState,
) -> Vec<ActionType> {
    let mut actions = Vec::new();

    // Check pattern matching first
    let pattern_action = decide_action(capture, rules);
    if !matches!(pattern_action, ActionType::SendKeys(ref keys) if keys.is_empty()) {
        actions.push(pattern_action);
        // Reset timeout state when pattern matches (activity detected)
        timeout_state.reset_activity();
    }

    // Check timeout rules
    let timeout_actions = check_timeout_rules(rules, timeout_state);
    actions.extend(timeout_actions);

    // If no actions, return empty action
    if actions.is_empty() {
        actions.push(ActionType::SendKeys(vec![]));
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn create_test_rule(pattern: &str, keys: Vec<String>) -> CompiledRule {
        CompiledRule {
            rule_type: RuleType::Pattern(Regex::new(pattern).unwrap()),
            action: ActionType::SendKeys(keys),
        }
    }

    fn create_workflow_rule(pattern: &str, workflow: String, args: Vec<String>) -> CompiledRule {
        CompiledRule {
            rule_type: RuleType::Pattern(Regex::new(pattern).unwrap()),
            action: ActionType::Workflow(workflow, args),
        }
    }

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
    fn test_decide_action_workflow() {
        let rules = vec![create_workflow_rule(
            r"fix\s+issue\s+(\d+)",
            "github_issue_fix".to_string(),
            vec!["${1}".to_string()],
        )];

        let action = decide_action("fix issue 456", &rules);
        assert_eq!(
            action,
            ActionType::Workflow("github_issue_fix".to_string(), vec!["456".to_string()])
        );
    }

    #[test]
    fn test_performance_100_rules() {
        use std::time::Instant;

        // Create 100 test rules that don't match our test input
        let rules: Vec<CompiledRule> = (0..100)
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
            create_test_rule("pattern", vec!["normal".to_string()]),
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
    fn test_decide_action_with_timeout() {
        let rules = vec![
            create_test_rule("hello", vec!["hello_response".to_string()]),
            create_timeout_rule("1s", vec!["timeout_response".to_string()]),
        ];

        let mut timeout_state = TimeoutState::new();

        // Test pattern matching resets timeout
        let actions = decide_action_with_timeout("hello world", &rules, &mut timeout_state);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["hello_response".to_string()])
        );

        // Set timeout condition
        timeout_state.last_activity = Instant::now() - Duration::from_millis(1500);

        // Test timeout trigger
        let actions = decide_action_with_timeout("no match", &rules, &mut timeout_state);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ActionType::SendKeys(vec!["timeout_response".to_string()])
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
}
