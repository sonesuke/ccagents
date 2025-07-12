use crate::config::types::{ActionType, compile_action};
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use std::time::Duration;

// YAML structure for loading rules
#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(alias = "pattern")]
    pub when: Option<String>,
    #[serde(default)]
    pub diff_timeout: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

// Compiled structure for runtime use
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub rule_type: RuleType,
    pub action: ActionType,
}

#[derive(Debug, Clone)]
pub enum RuleType {
    Pattern(Regex),
    DiffTimeout(Duration),
}

impl Rule {
    pub fn compile(&self) -> Result<CompiledRule> {
        let rule_type = match (&self.when, &self.diff_timeout) {
            (Some(pattern), None) => {
                let regex = Regex::new(pattern)
                    .with_context(|| format!("Invalid regex pattern: {}", pattern))?;
                RuleType::Pattern(regex)
            }
            (None, Some(timeout_str)) => {
                let duration = parse_duration(timeout_str)?;
                RuleType::DiffTimeout(duration)
            }
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "Rule cannot have both 'when' and 'diff_timeout' fields"
                ));
            }
            (None, None) => {
                return Err(anyhow!(
                    "Rule must have either 'when' or 'diff_timeout' field"
                ));
            }
        };

        let action = compile_action(&self.action, &self.keys, &self.workflow, &self.args)?;

        Ok(CompiledRule { rule_type, action })
    }
}

/// Resolve capture group references in a template string
pub fn resolve_capture_groups(template: &str, captured_groups: &[String]) -> String {
    let mut result = template.to_string();
    for (i, group) in captured_groups.iter().enumerate() {
        let placeholder = format!("${{{}}}", i + 1);
        result = result.replace(&placeholder, group);
    }
    result
}

/// Resolve capture groups in a vector of strings
pub fn resolve_capture_groups_in_vec(
    templates: &[String],
    captured_groups: &[String],
) -> Vec<String> {
    templates
        .iter()
        .map(|template| resolve_capture_groups(template, captured_groups))
        .collect()
}

/// Parse duration string (e.g., "30s", "5m", "2h") into Duration
fn parse_duration(s: &str) -> Result<Duration> {
    if s.is_empty() {
        return Err(anyhow!("Empty duration string"));
    }

    let (num_str, unit) = if let Some(stripped) = s.strip_suffix('s') {
        (stripped, "s")
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, "m")
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, "h")
    } else {
        return Err(anyhow!("Duration must end with 's', 'm', or 'h': {}", s));
    };

    let num: u64 = num_str
        .parse()
        .map_err(|_| anyhow!("Invalid number in duration: {}", num_str))?;

    let duration = match unit {
        "s" => Duration::from_secs(num),
        "m" => Duration::from_secs(num * 60),
        "h" => Duration::from_secs(num * 3600),
        _ => unreachable!(),
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));

        assert!(parse_duration("").is_err());
        assert!(parse_duration("30").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("30x").is_err());
    }

    #[test]
    fn test_rule_compilation_pattern() {
        let rule = Rule {
            when: Some("test".to_string()),
            diff_timeout: None,
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
            workflow: None,
            args: vec![],
        };

        let compiled = rule.compile().unwrap();
        match compiled.rule_type {
            RuleType::Pattern(ref regex) => {
                assert_eq!(regex.as_str(), "test");
            }
            _ => panic!("Expected pattern rule type"),
        }
    }

    #[test]
    fn test_rule_compilation_diff_timeout() {
        let rule = Rule {
            when: None,
            diff_timeout: Some("5m".to_string()),
            action: Some("send_keys".to_string()),
            keys: vec!["timeout".to_string()],
            workflow: None,
            args: vec![],
        };

        let compiled = rule.compile().unwrap();
        match compiled.rule_type {
            RuleType::DiffTimeout(duration) => {
                assert_eq!(duration, Duration::from_secs(300));
            }
            _ => panic!("Expected diff timeout rule type"),
        }
    }

    #[test]
    fn test_rule_compilation_both_fields_error() {
        let rule = Rule {
            when: Some("test".to_string()),
            diff_timeout: Some("5m".to_string()),
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
            workflow: None,
            args: vec![],
        };

        assert!(rule.compile().is_err());
    }

    #[test]
    fn test_rule_compilation_no_fields_error() {
        let rule = Rule {
            when: None,
            diff_timeout: None,
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
            workflow: None,
            args: vec![],
        };

        assert!(rule.compile().is_err());
    }
}
