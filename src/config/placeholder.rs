use crate::config::types::ActionType;

/// Resolve ${1} placeholders in action with source line content
pub fn resolve_source_placeholders(action: &ActionType, value: &str) -> ActionType {
    match action {
        ActionType::SendKeys(keys) => {
            let resolved_keys = keys.iter().map(|key| key.replace("${1}", value)).collect();
            ActionType::SendKeys(resolved_keys)
        }
    }
}
