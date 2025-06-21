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
        if let Some(captures) = rule.regex.captures(capture) {
            let mut args = rule.args.clone();

            // Extract capture groups and add them to args
            // Skip the first capture (index 0) which is the full match
            for i in 1..captures.len() {
                if let Some(captured) = captures.get(i) {
                    args.push(captured.as_str().to_string());
                }
            }

            return (rule.command.clone(), args);
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
            create_test_rule(10, r"issue\s+(\d+)", CmdKind::Entry, vec![]),
            create_test_rule(20, r"cancel", CmdKind::Cancel, vec![]),
        ];

        let (command, args) = decide_cmd("issue 123", &rules);
        assert_eq!(command, CmdKind::Entry);
        assert_eq!(args, vec!["123"]); // Should capture the issue number
    }

    #[test]
    fn test_decide_cmd_priority_ordering() {
        let rules = vec![
            create_test_rule(10, r"test", CmdKind::Entry, vec!["high".to_string()]),
            create_test_rule(20, r"test", CmdKind::Cancel, vec!["low".to_string()]),
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
            r"cancel",
            CmdKind::Cancel,
            vec!["static".to_string()],
        )];

        let (command, args) = decide_cmd("cancel", &rules);
        assert_eq!(command, CmdKind::Cancel);
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
