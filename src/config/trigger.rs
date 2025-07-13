use crate::config::types::{ActionType, parse_action};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// YAML structure for loading trigger configuration
#[derive(Debug, Deserialize)]
pub struct TriggerConfig {
    pub name: String,
    #[serde(alias = "trigger")]
    pub event: String,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
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
    UserCommand(String),
    Periodic { interval: std::time::Duration },
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
            TriggerType::UserCommand(config.event.clone())
        };

        let action = parse_action(&config.action, &config.keys, &config.workflow, &config.args)?;

        Ok(Self {
            name: config.name,
            trigger,
            action,
            source: config.source,
            dedupe: config.dedupe,
        })
    }
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

/// Configuration for trigger system
#[derive(Clone)]
pub struct TriggerManager {
    triggers: Arc<RwLock<Vec<Trigger>>>,
}

impl TriggerManager {
    /// Create new trigger manager
    pub fn new(triggers: Vec<Trigger>) -> Self {
        Self {
            triggers: Arc::new(RwLock::new(triggers)),
        }
    }

    /// Get startup triggers (on_start triggers)
    pub async fn get_on_start_triggers(&self) -> Vec<Trigger> {
        let triggers = self.triggers.read().await;
        triggers
            .iter()
            .filter(|trigger| trigger.trigger == TriggerType::OnStart)
            .cloned()
            .collect()
    }

    /// Get periodic triggers (periodic triggers)
    pub async fn get_periodic_triggers(&self) -> Vec<Trigger> {
        let triggers = self.triggers.read().await;
        triggers
            .iter()
            .filter(|trigger| matches!(trigger.trigger, TriggerType::Periodic { .. }))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
