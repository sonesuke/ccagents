use crate::ruler::types::{compile_action, ActionType};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::time::Duration;

// YAML structure for loading entries (triggers)
#[derive(Debug, Deserialize)]
pub struct Entry {
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
    pub queue: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
}

// Compiled structure for runtime use
#[derive(Debug, Clone)]
pub struct CompiledEntry {
    pub name: String,
    pub trigger: TriggerType,
    pub action: ActionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerType {
    OnStart,
    UserCommand(String),
    Periodic { interval: std::time::Duration },
    Enqueue { queue_name: String },
}

impl Entry {
    pub fn compile(&self) -> Result<CompiledEntry> {
        let trigger = if self.event.starts_with("timer:") {
            let duration_str = self
                .event
                .strip_prefix("timer:")
                .ok_or_else(|| anyhow::anyhow!("Invalid timer format"))?
                .to_string();
            let interval = parse_duration(&duration_str)?;
            TriggerType::Periodic { interval }
        } else if self.event.starts_with("queue:") {
            let queue_name = self
                .event
                .strip_prefix("queue:")
                .ok_or_else(|| anyhow::anyhow!("Invalid queue trigger format"))?
                .to_string();
            TriggerType::Enqueue { queue_name }
        } else if self.event == "startup" {
            TriggerType::OnStart
        } else {
            TriggerType::UserCommand(self.event.clone())
        };

        let action = compile_action(
            &self.action,
            &self.keys,
            &self.workflow,
            &self.args,
            &self.queue,
            &self.command,
        )?;

        Ok(CompiledEntry {
            name: self.name.clone(),
            trigger,
            action,
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
