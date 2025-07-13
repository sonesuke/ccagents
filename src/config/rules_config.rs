use crate::config::helper::{ActionType, parse_action};
use crate::config::helper::parse_duration;
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use std::convert::TryFrom;
use std::time::Duration;

// YAML structure for loading rules
#[derive(Debug, Deserialize, Clone)]
pub struct RuleConfig {
    pub when: Option<String>,
    #[serde(default)]
    pub diff_timeout: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
}

// Parsed and validated structure for runtime use
#[derive(Debug, Clone)]
pub struct Rule {
    pub rule_type: RuleType,
    pub action: ActionType,
}

#[derive(Debug, Clone)]
pub enum RuleType {
    When(Regex),
    DiffTimeout(Duration),
}

impl TryFrom<RuleConfig> for Rule {
    type Error = anyhow::Error;

    fn try_from(config: RuleConfig) -> Result<Self> {
        let rule_type = match (&config.when, &config.diff_timeout) {
            (Some(pattern), None) => {
                let regex = Regex::new(pattern)
                    .with_context(|| format!("Invalid regex pattern: {}", pattern))?;
                RuleType::When(regex)
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

        let action = parse_action(&config.action, &config.keys)?;

        Ok(Self { rule_type, action })
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;


    #[test]
    fn test_rule_try_from_pattern() {
        let rule = RuleConfig {
            when: Some("test".to_string()),
            diff_timeout: None,
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
        };

        let rule = Rule::try_from(rule).unwrap();
        match rule.rule_type {
            RuleType::When(ref regex) => {
                assert_eq!(regex.as_str(), "test");
            }
            _ => panic!("Expected when rule type"),
        }
    }

    #[test]
    fn test_rule_try_from_diff_timeout() {
        let rule = RuleConfig {
            when: None,
            diff_timeout: Some("5m".to_string()),
            action: Some("send_keys".to_string()),
            keys: vec!["timeout".to_string()],
        };

        let rule = Rule::try_from(rule).unwrap();
        match rule.rule_type {
            RuleType::DiffTimeout(duration) => {
                assert_eq!(duration, Duration::from_secs(300));
            }
            _ => panic!("Expected diff timeout rule type"),
        }
    }

    #[test]
    fn test_rule_try_from_both_fields_error() {
        let rule = RuleConfig {
            when: Some("test".to_string()),
            diff_timeout: Some("5m".to_string()),
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
        };

        assert!(Rule::try_from(rule).is_err());
    }

    #[test]
    fn test_rule_try_from_no_fields_error() {
        let rule = RuleConfig {
            when: None,
            diff_timeout: None,
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
        };

        assert!(Rule::try_from(rule).is_err());
    }
}
