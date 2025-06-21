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
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    // Legacy support for existing rules
    #[serde(default)]
    pub command: Option<String>,
}

// Compiled structures for runtime use
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub priority: u32,
    pub regex: Regex,
    pub action: ActionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    SendKeys(Vec<String>),
    Workflow(String, Vec<String>),
    // Legacy support during transition
    Legacy(CmdKind, Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CmdKind {
    Entry,
    Cancel,
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

    // Determine action type from rule - prioritize new action field over legacy command field
    let action = if let Some(action_type) = &rule.action {
        // New action-based rules
        match action_type.as_str() {
            "send_keys" => {
                if rule.keys.is_empty() {
                    anyhow::bail!("send_keys action requires 'keys' field");
                }
                ActionType::SendKeys(rule.keys.clone())
            }
            "workflow" => {
                let workflow_name = rule
                    .workflow
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("workflow action requires 'workflow' field"))?;
                ActionType::Workflow(workflow_name.clone(), rule.args.clone())
            }
            _ => anyhow::bail!("Unknown action type: {}", action_type),
        }
    } else if let Some(command) = &rule.command {
        // Legacy command support
        let cmd_kind = match command.as_str() {
            "entry" => CmdKind::Entry,
            "cancel" => CmdKind::Cancel,
            "resume" => CmdKind::Resume,
            _ => anyhow::bail!("Unknown legacy command: {}", command),
        };
        ActionType::Legacy(cmd_kind, rule.args.clone())
    } else {
        anyhow::bail!("Rule must have either 'action' or 'command' field");
    };

    Ok(CompiledRule {
        priority: rule.priority,
        regex,
        action,
    })
}

/// Resolve capture group references in a template string
pub fn resolve_capture_groups(template: &str, captured_groups: &[String]) -> String {
    let mut result = template.to_string();
    for (i, group) in captured_groups.iter().enumerate() {
        let placeholder = format!("${{{}}}", i + 1);
        result = result.replace(&placeholder, group);
    }
    result
}

/// Resolve capture groups in a vector of strings
pub fn resolve_capture_groups_in_vec(
    templates: &[String],
    captured_groups: &[String],
) -> Vec<String> {
    templates
        .iter()
        .map(|template| resolve_capture_groups(template, captured_groups))
        .collect()
}
