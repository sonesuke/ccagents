use crate::ruler::types::{ActionType, compile_action};
use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;

// YAML structure for loading rules
#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(alias = "pattern")]
    pub when: String,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub queue: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
}

// Compiled structure for runtime use
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub regex: Regex,
    pub action: ActionType,
}

impl Rule {
    pub fn compile(&self) -> Result<CompiledRule> {
        let regex = Regex::new(&self.when)
            .with_context(|| format!("Invalid regex pattern: {}", self.when))?;

        let action = compile_action(
            &self.action,
            &self.keys,
            &self.workflow,
            &self.args,
            &self.queue,
            &self.command,
        )?;

        Ok(CompiledRule { regex, action })
    }
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

/// Resolve <task> placeholder in a template string with a task value
pub fn resolve_task_placeholder(template: &str, task_value: &str) -> String {
    template.replace("<task>", task_value)
}

/// Resolve <task> placeholder in a vector of strings
pub fn resolve_task_placeholder_in_vec(templates: &[String], task_value: &str) -> Vec<String> {
    templates
        .iter()
        .map(|template| resolve_task_placeholder(template, task_value))
        .collect()
}
