use crate::rule_engine::{CmdKind, CompiledRule};

/// Matches capture text against compiled rules and returns the appropriate command and arguments.
///
/// This function iterates through rules in priority order (already sorted by load_rules)
/// and returns the first matching rule's command and arguments.
/// If no rules match, returns (CmdKind::Resume, vec![]) as the default.
///
/// # Arguments
/// * `capture` - The text to match against rule patterns
/// * `rules` - Slice of compiled rules, assumed to be sorted by priority
///
/// # Returns
/// A tuple of (CmdKind, Vec<String>) representing the command and its arguments
///
/// # Performance
/// Early termination on first match ensures optimal performance.
/// Should complete within 1ms for 100 rules with typical patterns.
pub fn decide_cmd(capture: &str, rules: &[CompiledRule]) -> (CmdKind, Vec<String>) {
    for rule in rules {
        if rule.regex.is_match(capture) {
            return (rule.command.clone(), rule.args.clone());
        }
    }

    // Default case: no rules matched
    (CmdKind::Resume, vec![])
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
            command,
            args,
        }
    }

    #[test]
    fn test_decide_cmd_exact_match() {
        let rules = vec![
            create_test_rule(10, r"issue\s+(\d+)", CmdKind::SolveIssue, vec![]),
            create_test_rule(20, r"cancel", CmdKind::Cancel, vec![]),
        ];

        let (command, args) = decide_cmd("issue 123", &rules);
        assert_eq!(command, CmdKind::SolveIssue);
        assert!(args.is_empty());
    }

    #[test]
    fn test_decide_cmd_priority_ordering() {
        let rules = vec![
            create_test_rule(10, r"test", CmdKind::SolveIssue, vec!["high".to_string()]),
            create_test_rule(20, r"test", CmdKind::Cancel, vec!["low".to_string()]),
        ];

        // Should match the first rule (higher priority - lower number)
        let (command, args) = decide_cmd("test", &rules);
        assert_eq!(command, CmdKind::SolveIssue);
        assert_eq!(args, vec!["high"]);
    }

    #[test]
    fn test_decide_cmd_no_match() {
        let rules = vec![
            create_test_rule(10, r"issue\s+(\d+)", CmdKind::SolveIssue, vec![]),
            create_test_rule(20, r"cancel", CmdKind::Cancel, vec![]),
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
            CmdKind::SolveIssue,
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
