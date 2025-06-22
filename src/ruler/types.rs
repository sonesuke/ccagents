use anyhow::Result;

// Shared action types for both entries and rules
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    SendKeys(Vec<String>),
    Workflow(String, Vec<String>),
    Enqueue { queue: String, command: String },
    EnqueueDedupe { queue: String, command: String },
}

/// Compile action from YAML fields into ActionType
pub fn compile_action(
    action: &Option<String>,
    keys: &[String],
    workflow: &Option<String>,
    args: &[String],
    queue: &Option<String>,
    command: &Option<String>,
) -> Result<ActionType> {
    let action = if let Some(action_type) = action {
        match action_type.as_str() {
            "send_keys" => {
                if keys.is_empty() {
                    anyhow::bail!("send_keys action requires 'keys' field");
                }
                ActionType::SendKeys(keys.to_vec())
            }
            "workflow" => {
                let workflow_name = workflow
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("workflow action requires 'workflow' field"))?;
                ActionType::Workflow(workflow_name.clone(), args.to_vec())
            }
            "enqueue" => {
                let queue_name = queue
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("enqueue action requires 'queue' field"))?;
                let command_str = command
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("enqueue action requires 'command' field"))?;
                ActionType::Enqueue {
                    queue: queue_name.clone(),
                    command: command_str.clone(),
                }
            }
            "enqueue_dedupe" => {
                let queue_name = queue.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("enqueue_dedupe action requires 'queue' field")
                })?;
                let command_str = command.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("enqueue_dedupe action requires 'command' field")
                })?;
                ActionType::EnqueueDedupe {
                    queue: queue_name.clone(),
                    command: command_str.clone(),
                }
            }
            _ => anyhow::bail!("Unknown action type: {}", action_type),
        }
    } else {
        anyhow::bail!("Must have 'action' field");
    };

    Ok(action)
}
