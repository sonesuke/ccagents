use crate::ruler::rule::CompiledRule;
use crate::ruler::rule::{resolve_capture_groups, resolve_capture_groups_in_vec};
use crate::ruler::types::ActionType;

/// Matches capture text against compiled rules and returns the appropriate action.
///
/// This function iterates through rules in priority order (already sorted by load_rules)
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
    for rule in rules {
        if let Some(captures) = rule.regex.captures(capture) {
            // Extract capture groups
            let captured_groups: Vec<String> = captures
                .iter()
                .skip(1) // Skip the full match (index 0)
                .filter_map(|m| m.map(|m| m.as_str().to_string()))
                .collect();

            // Resolve capture groups in the action
            let resolved_action = match &rule.action {
                ActionType::SendKeys(keys) => {
                    ActionType::SendKeys(resolve_capture_groups_in_vec(keys, &captured_groups))
                }
                ActionType::Workflow(workflow, args) => {
                    let resolved_args = resolve_capture_groups_in_vec(args, &captured_groups);
                    ActionType::Workflow(workflow.clone(), resolved_args)
                }
                ActionType::Enqueue { queue, command } => {
                    let resolved_queue = resolve_capture_groups(queue, &captured_groups);
                    let resolved_command = resolve_capture_groups(command, &captured_groups);
                    ActionType::Enqueue {
                        queue: resolved_queue,
                        command: resolved_command,
                    }
                }
            };

            return resolved_action;
        }
    }

    // Default case: no rules matched
    ActionType::SendKeys(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn create_test_rule(pattern: &str, keys: Vec<String>) -> CompiledRule {
        CompiledRule {
            regex: Regex::new(pattern).unwrap(),
            action: ActionType::SendKeys(keys),
        }
    }

    fn create_workflow_rule(pattern: &str, workflow: String, args: Vec<String>) -> CompiledRule {
        CompiledRule {
            regex: Regex::new(pattern).unwrap(),
            action: ActionType::Workflow(workflow, args),
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
}
