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

    let mut compiled_entries = Vec::new();
    for entry in config_file.agents.triggers {
        let compiled = entry
            .compile()
            .with_context(|| format!("Failed to compile trigger: {}", entry.name))?;
        compiled_entries.push(compiled);
    }

    let mut compiled_rules = Vec::new();
    for rule in config_file.agents.rules {
        let compiled = rule.compile().with_context(|| {
            format!(
                "Failed to compile rule with pattern: {:?}",
                rule.when.as_deref().unwrap_or("timeout")
            )
        })?;
        compiled_rules.push(compiled);
    }

    // Rules are processed in order (no sorting needed - line order = priority)

    let monitor_config = MonitorConfig {
        web_ui: config_file.web_ui,
        agents: AgentsConfig {
            pool: config_file.agents.pool,
        },
    };

    Ok((compiled_entries, compiled_rules, monitor_config))
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
