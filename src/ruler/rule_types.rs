use regex::Regex;
use serde::Deserialize;

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
    Resume,
}
