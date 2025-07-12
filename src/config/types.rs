use anyhow::Result;

// Shared action types for both entries and rules
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    SendKeys(Vec<String>),
}

/// Compile action from YAML fields into ActionType
pub fn compile_action(
    action: &Option<String>,
    keys: &[String],
    _workflow: &Option<String>,
    _args: &[String],
) -> Result<ActionType> {
    let action = if let Some(action_type) = action {
        match action_type.as_str() {
            "send_keys" => {
                if keys.is_empty() {
                    anyhow::bail!("send_keys action requires 'keys' field");
                }
                ActionType::SendKeys(keys.to_vec())
            }
            _ => anyhow::bail!("Unknown action type: {}", action_type),
        }
    } else {
        anyhow::bail!("Must have 'action' field");
    };

    Ok(action)
}
