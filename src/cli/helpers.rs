use crate::agent;
use crate::queue::{QueueExecutor, SharedQueueManager};
use crate::ruler;
use anyhow::Result;

/// Execute a periodic entry action (with agent context)
pub async fn execute_periodic_entry(
    entry: &ruler::entry::CompiledEntry,
    queue_manager: &SharedQueueManager,
    agent: Option<&agent::Agent>,
) -> Result<()> {
    match &entry.action {
        ruler::types::ActionType::SendKeys(keys) => {
            if let Some(agent) = agent {
                println!(
                    "🤖 Executing periodic entry '{}' → Sending: {:?}",
                    entry.name, keys
                );
                for key in keys {
                    agent.send_keys(key).await?;
                }
            } else {
                println!(
                    "⚠️ Periodic entry '{}' has SendKeys action - skipping (no agent context)",
                    entry.name
                );
            }
        }
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            println!(
                "🔄 Executing periodic entry '{}' → Workflow: {} {:?}",
                entry.name, workflow_name, args
            );
            // TODO: Implement workflow execution
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            println!(
                "📦 Executing periodic entry '{}' → Enqueue to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue(queue, command).await?;
            println!("✅ Enqueued {} items to queue '{}'", count, queue);
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            println!(
                "📦 Executing periodic entry '{}' → EnqueueDedupe to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue_dedupe(queue, command).await?;
            println!(
                "✅ Enqueued {} new items to dedupe queue '{}'",
                count, queue
            );
        }
    }
    Ok(())
}

/// Execute an entry action using the appropriate mechanism
pub async fn execute_entry_action(
    agent: &agent::Agent,
    entry: &ruler::entry::CompiledEntry,
    queue_manager: &SharedQueueManager,
) -> Result<()> {
    match &entry.action {
        ruler::types::ActionType::SendKeys(keys) => {
            println!("🤖 Executing entry '{}' → Sending: {:?}", entry.name, keys);
            for key in keys {
                if key == "\\r" || key == "\r" {
                    if let Err(e) = agent.send_keys("\r").await {
                        eprintln!("❌ Error sending key: {}", e);
                    }
                } else if let Err(e) = agent.send_keys(key).await {
                    eprintln!("❌ Error sending key: {}", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            println!(
                "🔄 Executing entry '{}' → Workflow: {} {:?}",
                entry.name, workflow_name, args
            );
            // TODO: Implement workflow execution
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            println!(
                "📦 Executing entry '{}' → Enqueue to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue(queue, command).await?;
            println!("✅ Enqueued {} items to queue '{}'", count, queue);
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            println!(
                "📦 Executing entry '{}' → EnqueueDedupe to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue_dedupe(queue, command).await?;
            println!(
                "✅ Enqueued {} new items to dedupe queue '{}'",
                count, queue
            );
        }
    }
    Ok(())
}

/// Resolve <task> placeholders in entry action with actual task value
pub fn resolve_entry_task_placeholders(
    entry: &ruler::entry::CompiledEntry,
    task_value: &str,
) -> ruler::entry::CompiledEntry {
    let resolved_action = match &entry.action {
        ruler::types::ActionType::SendKeys(keys) => ruler::types::ActionType::SendKeys(
            ruler::rule::resolve_task_placeholder_in_vec(keys, task_value),
        ),
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            let resolved_workflow =
                ruler::rule::resolve_task_placeholder(workflow_name, task_value);
            let resolved_args = ruler::rule::resolve_task_placeholder_in_vec(args, task_value);
            ruler::types::ActionType::Workflow(resolved_workflow, resolved_args)
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            let resolved_queue = ruler::rule::resolve_task_placeholder(queue, task_value);
            let resolved_command = ruler::rule::resolve_task_placeholder(command, task_value);
            ruler::types::ActionType::Enqueue {
                queue: resolved_queue,
                command: resolved_command,
            }
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            let resolved_queue = ruler::rule::resolve_task_placeholder(queue, task_value);
            let resolved_command = ruler::rule::resolve_task_placeholder(command, task_value);
            ruler::types::ActionType::EnqueueDedupe {
                queue: resolved_queue,
                command: resolved_command,
            }
        }
    };

    ruler::entry::CompiledEntry {
        name: entry.name.clone(),
        trigger: entry.trigger.clone(),
        action: resolved_action,
        concurrency: entry.concurrency,
    }
}
