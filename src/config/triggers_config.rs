use crate::config::helper::parse_duration;
use crate::config::helper::{ActionType, parse_action};
use anyhow::Result;
use serde::Deserialize;
use std::convert::TryFrom;
use std::time::Duration;

// YAML structure for loading trigger configuration
#[derive(Debug, Deserialize, Clone)]
pub struct TriggerConfig {
    pub name: String,
    pub event: String,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub dedupe: bool,
}

// Parsed and validated structure for runtime use
#[derive(Debug, Clone)]
pub struct Trigger {
    pub name: String,
    pub trigger: TriggerType,
    pub action: ActionType,
    pub source: Option<String>,
    #[allow(dead_code)] // Will be used for future deduplication functionality
    pub dedupe: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerType {
    OnStart,
    Periodic { interval: Duration },
}

impl TryFrom<TriggerConfig> for Trigger {
    type Error = anyhow::Error;

    fn try_from(config: TriggerConfig) -> Result<Self> {
        let trigger = if config.event.starts_with("timer:") {
            let duration_str = config
                .event
                .strip_prefix("timer:")
                .ok_or_else(|| anyhow::anyhow!("Invalid timer format"))?
                .to_string();
            let interval = parse_duration(&duration_str)?;
            TriggerType::Periodic { interval }
        } else if config.event == "startup" {
            TriggerType::OnStart
        } else {
            return Err(anyhow::anyhow!("Unknown event type: {}", config.event));
        };

        let action = parse_action(&config.action, &config.keys)?;

        Ok(Self {
            name: config.name,
            trigger,
            action,
            source: config.source,
            dedupe: config.dedupe,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_trigger_from_startup_config() {
        let config = TriggerConfig {
            name: "test_startup".to_string(),
            event: "startup".to_string(),
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
            source: None,
            dedupe: false,
        };

        let trigger = Trigger::try_from(config).unwrap();
        assert_eq!(trigger.name, "test_startup");
        assert_eq!(trigger.trigger, TriggerType::OnStart);
        match trigger.action {
            ActionType::SendKeys(keys) => assert_eq!(keys, vec!["hello"]),
        }
    }

    #[test]
    fn test_trigger_from_timer_config() {
        let config = TriggerConfig {
            name: "test_timer".to_string(),
            event: "timer:30s".to_string(),
            action: Some("send_keys".to_string()),
            keys: vec!["tick".to_string()],
            source: Some("source1".to_string()),
            dedupe: true,
        };

        let trigger = Trigger::try_from(config).unwrap();
        assert_eq!(trigger.name, "test_timer");
        assert_eq!(
            trigger.trigger,
            TriggerType::Periodic {
                interval: Duration::from_secs(30)
            }
        );
        assert_eq!(trigger.source, Some("source1".to_string()));
        assert_eq!(trigger.dedupe, true);
    }

    #[test]
    fn test_trigger_from_invalid_event() {
        let config = TriggerConfig {
            name: "test_invalid".to_string(),
            event: "invalid_event".to_string(),
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
            source: None,
            dedupe: false,
        };

        assert!(Trigger::try_from(config).is_err());
    }

    #[test]
    fn test_trigger_from_invalid_timer_format() {
        let config = TriggerConfig {
            name: "test_bad_timer".to_string(),
            event: "timer:invalid".to_string(),
            action: Some("send_keys".to_string()),
            keys: vec!["hello".to_string()],
            source: None,
            dedupe: false,
        };

        assert!(Trigger::try_from(config).is_err());
    }

    #[test]
    fn test_trigger_type_equality() {
        assert_eq!(TriggerType::OnStart, TriggerType::OnStart);
        assert_eq!(
            TriggerType::Periodic {
                interval: Duration::from_secs(30)
            },
            TriggerType::Periodic {
                interval: Duration::from_secs(30)
            }
        );
        assert_ne!(
            TriggerType::OnStart,
            TriggerType::Periodic {
                interval: Duration::from_secs(30)
            }
        );
    }
}
