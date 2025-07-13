use crate::config::rule::RuleConfig;
use crate::config::trigger::TriggerConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AgentsConfig {
    #[serde(default = "default_pool_size")]
    pub pool: usize,
}

// Extended agents config that includes triggers and rules
#[derive(Debug, Deserialize, Default)]
pub struct FullAgentsConfig {
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
        }
    }
}

fn default_pool_size() -> usize {
    1
}
