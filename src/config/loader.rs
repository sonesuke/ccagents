use crate::config::agents::{AgentsConfig, FullAgentsConfig};
use crate::config::rule::Rule;
use crate::config::trigger::Trigger;
use crate::config::web_ui::WebUIConfig;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

// Configuration structure with separate web_ui and agents sections
#[derive(Debug, Deserialize, Default, Clone)]
pub struct MonitorConfig {
    #[serde(default)]
    pub web_ui: WebUIConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
}

// YAML structure for loading complete configuration
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub web_ui: WebUIConfig,
    #[serde(default)]
    pub agents: FullAgentsConfig,
}

/// Load configuration from a YAML file and compile entries and rules
pub fn load_config(path: &Path) -> Result<(Vec<Trigger>, Vec<Rule>, MonitorConfig)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config_file: ConfigFile =
        serde_yml::from_str(&content).with_context(|| "Failed to parse YAML config file")?;

    let mut triggers = Vec::new();
    for trigger_config in config_file.agents.triggers {
        let name = trigger_config.name.clone(); // Clone name before moving trigger_config
        let trigger = Trigger::try_from(trigger_config)
            .with_context(|| format!("Failed to parse trigger: {}", name))?;
        triggers.push(trigger);
    }

    let mut rules = Vec::new();
    for rule_config in config_file.agents.rules {
        let pattern = rule_config.when.as_deref().unwrap_or("timeout").to_string(); // Clone pattern before moving
        let rule = Rule::try_from(rule_config)
            .with_context(|| format!("Failed to parse rule with pattern: {:?}", pattern))?;
        rules.push(rule);
    }

    // Rules are processed in order (no sorting needed - line order = priority)

    let monitor_config = MonitorConfig {
        web_ui: config_file.web_ui,
        agents: AgentsConfig {
            pool: config_file.agents.pool,
        },
    };

    Ok((triggers, rules, monitor_config))
}

impl MonitorConfig {
    /// Get terminal dimensions for agents (same for all agents)
    pub fn get_agent_dimensions(&self, _index: usize) -> (u16, u16) {
        (self.web_ui.cols, self.web_ui.rows)
    }

    /// Get web UI base port (for backward compatibility)
    pub fn get_web_ui_port(&self) -> u16 {
        self.web_ui.base_port
    }

    /// Get agent pool size (for backward compatibility)
    pub fn get_agent_pool_size(&self) -> usize {
        self.agents.pool
    }
}
