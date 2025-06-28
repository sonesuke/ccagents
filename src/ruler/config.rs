use crate::ruler::entry::{CompiledEntry, Entry};
use crate::ruler::rule::{CompiledRule, Rule};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

// Monitor configuration for base port and other monitoring settings
#[derive(Debug, Deserialize)]
pub struct MonitorConfig {
    #[serde(default = "default_base_port")]
    pub base_port: u16,
    #[serde(default = "default_agent_pool_size")]
    pub agent_pool_size: usize,
    #[serde(default)]
    pub web_ui: WebUIConfig,
}

#[derive(Debug, Deserialize)]
pub struct WebUIConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_theme")]
    #[allow(dead_code)]  // Theme will be used in future theme support
    pub theme: String,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            base_port: default_base_port(),
            agent_pool_size: default_agent_pool_size(),
            web_ui: WebUIConfig::default(),
        }
    }
}

impl Default for WebUIConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            host: default_host(),
            theme: default_theme(),
        }
    }
}

fn default_base_port() -> u16 {
    9990
}

fn default_agent_pool_size() -> usize {
    1
}

fn default_enabled() -> bool {
    true
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_theme() -> String {
    "default".to_string()
}

// YAML structure for loading complete configuration
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub monitor: MonitorConfig,
}

/// Load configuration from a YAML file and compile entries and rules
pub fn load_config(path: &Path) -> Result<(Vec<CompiledEntry>, Vec<CompiledRule>, MonitorConfig)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config_file: ConfigFile =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse YAML config file")?;

    let mut compiled_entries = Vec::new();
    for entry in config_file.entries {
        let compiled = entry
            .compile()
            .with_context(|| format!("Failed to compile entry: {}", entry.name))?;
        compiled_entries.push(compiled);
    }

    let mut compiled_rules = Vec::new();
    for rule in config_file.rules {
        let compiled = rule
            .compile()
            .with_context(|| format!("Failed to compile rule with pattern: {}", rule.pattern))?;
        compiled_rules.push(compiled);
    }

    // Rules are processed in order (no sorting needed - line order = priority)

    Ok((compiled_entries, compiled_rules, config_file.monitor))
}
