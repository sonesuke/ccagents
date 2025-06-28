use crate::agent;
use crate::queue::{QueueExecutor, SharedQueueManager};
use crate::ruler;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
fn truncate_unicode_safe(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }

    // Find the last valid UTF-8 character boundary at or before max_len
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

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
            // TODO: Implement custom workflow execution if needed
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
    // DETAILED DEBUG LOGGING FOR ENTRY EXECUTION
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("pty_debug.log")
    {
        let _ = writeln!(file, "[{}] === EXECUTING ENTRY ACTION ===", timestamp);
        let _ = writeln!(file, "Entry name: {}", entry.name);
        let _ = writeln!(file, "Action type: {:?}", entry.action);
        let _ = writeln!(file, "---");
    }

    match &entry.action {
        ruler::types::ActionType::SendKeys(keys) => {
            println!("🤖 Executing entry '{}' → Sending: {:?}", entry.name, keys);

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("pty_debug.log")
            {
                let _ = writeln!(
                    file,
                    "[{}] SendKeys action with {} keys:",
                    timestamp,
                    keys.len()
                );
                for (i, key) in keys.iter().enumerate() {
                    let _ = writeln!(file, "  Key {}: {:?}", i, key);
                }
            }

            for key in keys {
                println!("📤 Sending individual key: {:?}", key);

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
            // TODO: Implement custom workflow execution if needed
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
    }
}

/// Execute a rule action
pub async fn execute_rule_action(
    action: &ruler::types::ActionType,
    agent: &agent::Agent,
    queue_manager: &SharedQueueManager,
) -> Result<()> {
    match action {
        ruler::types::ActionType::SendKeys(keys) => {
            if !keys.is_empty() {
                println!("🤖 EXECUTING RULE → Sending: {:?}", keys);
                println!(
                    "🕐 Timestamp: {}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );

                // Send the keys to the terminal
                for (i, key) in keys.iter().enumerate() {
                    println!("  📤 Sending key {}: {:?}", i + 1, key);
                    if key == "\\r" || key == "\r" {
                        if let Err(e) = agent.send_keys("\r").await {
                            eprintln!("❌ Error sending key: {}", e);
                        }
                    } else if let Err(e) = agent.send_keys(key).await {
                        eprintln!("❌ Error sending key: {}", e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }

                println!("✅ Rule execution completed, waiting 1000ms");
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        }
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            println!("🔄 Matched workflow: {} {:?}", workflow_name, args);
            // TODO: Implement custom workflow execution if needed
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            println!("📦 Matched enqueue to '{}': {}", queue, command);
            let executor = QueueExecutor::new(queue_manager.clone());
            match executor.execute_and_enqueue(queue, command).await {
                Ok(count) => {
                    println!("✅ Enqueued {} items to queue '{}'", count, queue);
                }
                Err(e) => {
                    eprintln!("❌ Error executing enqueue action: {}", e);
                }
            }
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            println!("📦 Matched enqueue_dedupe to '{}': {}", queue, command);
            let executor = QueueExecutor::new(queue_manager.clone());
            match executor.execute_and_enqueue_dedupe(queue, command).await {
                Ok(count) => {
                    println!(
                        "✅ Enqueued {} new items to dedupe queue '{}'",
                        count, queue
                    );
                }
                Err(e) => {
                    eprintln!("❌ Error executing enqueue_dedupe action: {}", e);
                }
            }
        }
    }
    Ok(())
}

/// Process direct command output (supports any command, not just Claude)
pub async fn process_direct_output(
    agent: &agent::Agent,
    ruler: &ruler::Ruler,
    queue_manager: &SharedQueueManager,
) -> Result<()> {
    // Check for direct command output and process
    while let Some(command_output) = agent.get_command_output().await {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("pattern_match_debug.log")
        {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let _ = writeln!(file, "\n[{}] === DIRECT COMMAND OUTPUT ===", timestamp);
            let _ = writeln!(
                file,
                "Source: {}",
                if command_output.is_stdout {
                    "stdout"
                } else {
                    "stderr"
                }
            );
            let _ = writeln!(file, "Content: {:?}", command_output.content);
            let _ = writeln!(file, "==> Will check rules for command output");
        }

        println!("📤 Command output: {}", command_output.content);

        let action = ruler
            .decide_action_for_capture(&command_output.content)
            .await;
        execute_rule_action(&action, agent, queue_manager).await?;
    }

    // Terminal diff detection removed - only direct command output monitoring remains
    Ok(())
}
