use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

// YAML structures for loading rules
#[derive(Debug, Deserialize)]
pub struct RuleFile {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub priority: u32,
    pub pattern: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

// Compiled structures for runtime use
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub priority: u32,
    pub regex: Regex,
    pub command: CmdKind,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CmdKind {
    Entry,
    Resume,
}

/// Load rules from a YAML file and compile them
pub fn load_rules(path: &Path) -> Result<Vec<CompiledRule>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read rules file: {}", path.display()))?;

    let rule_file: RuleFile =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse YAML rules file")?;

    let mut compiled_rules = Vec::new();
    for rule in rule_file.rules {
        let compiled = compile_rule(&rule)
            .with_context(|| format!("Failed to compile rule with pattern: {}", rule.pattern))?;
        compiled_rules.push(compiled);
    }

    // Sort by priority (ascending order - lower number = higher priority)
    compiled_rules.sort_by_key(|rule| rule.priority);

    Ok(compiled_rules)
}

fn compile_rule(rule: &Rule) -> Result<CompiledRule> {
    let regex = Regex::new(&rule.pattern)
        .with_context(|| format!("Invalid regex pattern: {}", rule.pattern))?;

    let command = match rule.command.as_str() {
        "entry" => CmdKind::Entry,
        "resume" => CmdKind::Resume,
        _ => anyhow::bail!("Unknown command: {}", rule.command),
    };

    Ok(CompiledRule {
        priority: rule.priority,
        regex,
        command,
        args: rule.args.clone(),
    })
}
