use crate::config::rules_config::RuleConfig;
use crate::config::triggers_config::TriggerConfig;
use serde::Deserialize;

// Agents config matching config.yaml structure
#[derive(Debug, Deserialize, Clone)]
pub struct AgentsConfig {
    #[serde(default = "default_pool_size")]
    pub pool: usize,
    #[serde(default)]
    pub triggers: Vec<TriggerConfig>,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}


impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            pool: default_pool_size(),
            triggers: Vec::new(),
            rules: Vec::new(),
        }
    }
}

fn default_pool_size() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_agents_config() {
        let config = AgentsConfig::default();
        assert_eq!(config.pool, 1);
        assert!(config.triggers.is_empty());
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_default_pool_size() {
        assert_eq!(default_pool_size(), 1);
    }

    #[test]
    fn test_agents_config_deserialization() {
        let yaml = r#"
pool: 2
triggers: []
rules: []
"#;
        let config: AgentsConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.pool, 2);
    }

    #[test]
    fn test_agents_config_partial_deserialization() {
        let yaml = r#"{}"#;
        let config: AgentsConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.pool, 1);
        assert!(config.triggers.is_empty());
        assert!(config.rules.is_empty());
    }
}
