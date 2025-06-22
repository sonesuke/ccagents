use crate::ruler::types::{compile_action, ActionType};
use anyhow::Result;
use serde::Deserialize;

// YAML structure for loading entries
#[derive(Debug, Deserialize)]
pub struct Entry {
    pub name: String,
    pub trigger: String,
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
pub struct CompiledEntry {
    pub name: String,
    pub trigger: TriggerType,
    pub action: ActionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerType {
    OnStart,
    UserCommand(String),
}

impl Entry {
    pub fn compile(&self) -> Result<CompiledEntry> {
        let trigger = match self.trigger.as_str() {
            "on_start" => TriggerType::OnStart,
            user_cmd => TriggerType::UserCommand(user_cmd.to_string()),
        };

        let action = compile_action(&self.action, &self.keys, &self.workflow, &self.args)?;

        Ok(CompiledEntry {
            name: self.name.clone(),
            trigger,
            action,
        })
    }
}