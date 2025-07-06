use crate::ruler::entry::{CompiledEntry, Entry};
use crate::ruler::rule::{CompiledRule, Rule};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

// Configuration structure with separate web_ui and agents sections
#[derive(Debug, Deserialize, Default)]
pub struct MonitorConfig {
    #[serde(default)]
    pub web_ui: WebUIConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AgentsConfig {
    #[serde(default = "default_pool_size")]
    pub pool: usize,
}

#[derive(Debug, Deserialize)]
pub struct WebUIConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_base_port")]
    pub base_port: u16,
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            pool: default_pool_size(),
        }
    }
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            host: default_host(),
            base_port: default_base_port(),
            cols: default_cols(),
            rows: default_rows(),
        }
    }
}

fn default_base_port() -> u16 {
    9990
}

fn default_pool_size() -> usize {
    1
}

fn default_enabled() -> bool {
    true
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

// YAML structure for loading complete configuration
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub web_ui: WebUIConfig,
    #[serde(default)]
    pub agents: FullAgentsConfig,
}

// Extended agents config that includes triggers and rules
#[derive(Debug, Deserialize, Default)]
pub struct FullAgentsConfig {
    #[serde(default = "default_pool_size")]
    pub pool: usize,
    #[serde(default)]
    pub triggers: Vec<Entry>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// Load configuration from a YAML file and compile entries and rules
pub fn load_config(path: &Path) -> Result<(Vec<CompiledEntry>, Vec<CompiledRule>, MonitorConfig)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config_file: ConfigFile =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse YAML config file")?;

    let mut compiled_entries = Vec::new();
    for entry in config_file.agents.triggers {
        let compiled = entry
            .compile()
            .with_context(|| format!("Failed to compile trigger: {}", entry.name))?;
        compiled_entries.push(compiled);
    }

    let mut compiled_rules = Vec::new();
    for rule in config_file.agents.rules {
        let compiled = rule
            .compile()
            .with_context(|| format!("Failed to compile rule with pattern: {}", rule.when))?;
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
