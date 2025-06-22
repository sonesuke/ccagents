use crate::ruler::types::{compile_action, ActionType};
use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;

// YAML structure for loading rules
#[derive(Debug, Deserialize)]
pub struct Rule {
    pub pattern: String,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

// Compiled structure for runtime use
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub regex: Regex,
    pub action: ActionType,
}

impl Rule {
    pub fn compile(&self) -> Result<CompiledRule> {
        let regex = Regex::new(&self.pattern)
            .with_context(|| format!("Invalid regex pattern: {}", self.pattern))?;

        let action = compile_action(&self.action, &self.keys, &self.workflow, &self.args)?;

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
