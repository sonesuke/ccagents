use crate::rule_engine::{resolve_capture_groups_in_vec, ActionType, CmdKind, CompiledRule};

/// Matches capture text against compiled rules and returns the appropriate action.
///
/// This function iterates through rules in priority order (already sorted by load_rules)
/// and returns the first matching rule's action with resolved capture groups.
/// If no rules match, returns ActionType::Legacy(CmdKind::Resume, vec![]) as the default.
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
                ActionType::Legacy(cmd_kind, args) => {
                    let mut resolved_args = args.clone();
                    // Add captured groups to legacy args for backward compatibility
                    resolved_args.extend(captured_groups);
                    ActionType::Legacy(cmd_kind.clone(), resolved_args)
                }
            };

            return resolved_action;
        }
    }

    // Default case: no rules matched
    ActionType::Legacy(CmdKind::Resume, vec![])
}

/// Legacy wrapper function for backward compatibility
///
/// This function maintains compatibility with existing code that expects
/// the old (CmdKind, Vec<String>) return type. It should be used during
/// the transition period.
pub fn decide_cmd(capture: &str, rules: &[CompiledRule]) -> (CmdKind, Vec<String>) {
    let action = decide_action(capture, rules);
    match action {
        ActionType::Legacy(cmd_kind, args) => (cmd_kind, args),
        _ => {
            // For non-legacy actions, return Resume to maintain compatibility
            // The new system should use decide_action directly
            (CmdKind::Resume, vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn create_test_rule(
        priority: u32,
        pattern: &str,
        command: CmdKind,
        args: Vec<String>,
    ) -> CompiledRule {
        CompiledRule {
            priority,
            regex: Regex::new(pattern).unwrap(),
            action: ActionType::Legacy(command, args),
        }
    }

    #[test]
    fn test_decide_cmd_exact_match() {
        let rules = vec![
            create_test_rule(10, r"issue\s+(\d+)", CmdKind::Entry, vec![]),
            create_test_rule(20, r"resume", CmdKind::Resume, vec![]),
        ];

        let (command, args) = decide_cmd("issue 123", &rules);
        assert_eq!(command, CmdKind::Entry);
        assert_eq!(args, vec!["123"]); // Should capture the issue number
    }

    #[test]
    fn test_decide_cmd_priority_ordering() {
        let rules = vec![
            create_test_rule(10, r"test", CmdKind::Entry, vec!["high".to_string()]),
            create_test_rule(20, r"test", CmdKind::Resume, vec!["low".to_string()]),
        ];

        // Should match the first rule (higher priority - lower number)
        let (command, args) = decide_cmd("test", &rules);
        assert_eq!(command, CmdKind::Entry);
        assert_eq!(args, vec!["high"]);
    }

    #[test]
    fn test_decide_cmd_no_match() {
        let rules = vec![
            create_test_rule(10, r"issue\s+(\d+)", CmdKind::Entry, vec![]),
            create_test_rule(20, r"resume", CmdKind::Resume, vec![]),
        ];

        let (command, args) = decide_cmd("no matching pattern here", &rules);
        assert_eq!(command, CmdKind::Resume);
        assert!(args.is_empty());
    }

    #[test]
    fn test_decide_cmd_empty_capture() {
        let rules = vec![create_test_rule(
            10,
            r"issue\s+(\d+)",
            CmdKind::Entry,
            vec![],
        )];

        let (command, args) = decide_cmd("", &rules);
        assert_eq!(command, CmdKind::Resume);
        assert!(args.is_empty());
    }

    #[test]
    fn test_decide_cmd_empty_rules() {
        let (command, args) = decide_cmd("any text", &[]);
        assert_eq!(command, CmdKind::Resume);
        assert!(args.is_empty());
    }

    #[test]
    fn test_decide_cmd_capture_groups() {
        let rules = vec![create_test_rule(
            10,
            r"deploy\s+(\w+)\s+to\s+(\w+)",
            CmdKind::Entry,
            vec!["static".to_string()],
        )];

        let (command, args) = decide_cmd("deploy app to production", &rules);
        assert_eq!(command, CmdKind::Entry);
        assert_eq!(args, vec!["static", "app", "production"]); // static + captured groups
    }

    #[test]
    fn test_decide_cmd_no_capture_groups() {
        let rules = vec![create_test_rule(
            20,
            r"resume",
            CmdKind::Resume,
            vec!["static".to_string()],
        )];

        let (command, args) = decide_cmd("resume", &rules);
        assert_eq!(command, CmdKind::Resume);
        assert_eq!(args, vec!["static"]); // Only static args, no capture groups
    }

    #[test]
    fn test_performance_100_rules() {
        use std::time::Instant;

        // Create 100 test rules that don't match our test input
        let rules: Vec<CompiledRule> = (0..100)
            .map(|i| create_test_rule(i, &format!("unique_pattern_{}", i), CmdKind::Resume, vec![]))
            .collect();

        let start = Instant::now();
        let (command, _) = decide_cmd("non-matching test input", &rules);
        let duration = start.elapsed();

        assert_eq!(command, CmdKind::Resume);
        assert!(
            duration.as_millis() < 100,
            "Should complete within 100ms for 100 rules, took {}ms",
            duration.as_millis()
        );
    }
}
