use crate::ruler::entry::{CompiledEntry, Entry};
use crate::ruler::rule::{CompiledRule, Rule};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

// YAML structure for loading complete configuration
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// Load configuration from a YAML file and compile entries and rules
pub fn load_config(path: &Path) -> Result<(Vec<CompiledEntry>, Vec<CompiledRule>)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config_file: ConfigFile =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse YAML config file")?;

    let mut compiled_entries = Vec::new();
    for entry in config_file.entries {
        let compiled = entry.compile()
            .with_context(|| format!("Failed to compile entry: {}", entry.name))?;
        compiled_entries.push(compiled);
    }

    let mut compiled_rules = Vec::new();
    for rule in config_file.rules {
        let compiled = rule.compile()
            .with_context(|| format!("Failed to compile rule with pattern: {}", rule.pattern))?;
        compiled_rules.push(compiled);
    }

    // Rules are processed in order (no sorting needed - line order = priority)

    Ok((compiled_entries, compiled_rules))
}

/// Legacy function for backward compatibility
pub fn load_rules(path: &Path) -> Result<Vec<CompiledRule>> {
    let (_, rules) = load_config(path)?;
    Ok(rules)
}