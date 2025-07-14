use anyhow::{Result, anyhow};
use std::time::Duration;

// Shared action types for both entries and rules
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    SendKeys(Vec<String>),
}

/// Parse duration string (e.g., "30s", "5m", "2h") into Duration
pub fn parse_duration(s: &str) -> Result<Duration> {
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

/// Parse and validate action from YAML fields into ActionType
pub fn parse_action(action: &Option<String>, keys: &[String]) -> Result<ActionType> {
    let action = if let Some(action_type) = action {
        match action_type.as_str() {
            "send_keys" => {
                if keys.is_empty() {
                    anyhow::bail!("send_keys action requires 'keys' field");
                }
                ActionType::SendKeys(keys.to_vec())
            }
            _ => anyhow::bail!("Unknown action type: {}", action_type),
        }
    } else {
        anyhow::bail!("Must have 'action' field");
    };

    Ok(action)
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

    #[test]
    fn test_parse_action_send_keys() {
        let action = Some("send_keys".to_string());
        let keys = vec!["hello".to_string(), "world".to_string()];
        let result = parse_action(&action, &keys).unwrap();
        assert_eq!(
            result,
            ActionType::SendKeys(vec!["hello".to_string(), "world".to_string()])
        );
    }

    #[test]
    fn test_parse_action_send_keys_empty_keys() {
        let action = Some("send_keys".to_string());
        let keys = vec![];
        let result = parse_action(&action, &keys);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_action_no_action() {
        let action = None;
        let keys = vec!["hello".to_string()];
        let result = parse_action(&action, &keys);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_action_unknown_action() {
        let action = Some("unknown_action".to_string());
        let keys = vec!["hello".to_string()];
        let result = parse_action(&action, &keys);
        assert!(result.is_err());
    }
}
