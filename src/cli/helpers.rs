use crate::agent;
use crate::queue::SharedQueueManager;
use crate::ruler;
use anyhow::Result;
use std::collections::HashSet;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Execute a periodic entry action (with agent context)
pub async fn execute_periodic_entry(
    entry: &ruler::entry::CompiledEntry,
    _queue_manager: &SharedQueueManager,
    agent: Option<&agent::Agent>,
) -> Result<()> {
    // If there's a source command, execute it first and process its output
    if let Some(source) = &entry.source {
        println!(
            "📦 Executing periodic entry '{}' → Source: {}",
            entry.name, source
        );

        // Execute the source command
        let output = Command::new("sh").arg("-c").arg(source).output()?;

        if !output.status.success() {
            eprintln!(
                "❌ Source command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.to_string())
            .collect();

        println!("✅ Source command produced {} lines", lines.len());

        // Process each line with deduplication if needed
        let mut seen = HashSet::new();
        let mut processed = 0;

        for line in lines {
            // Skip if deduplication is enabled and we've seen this line
            if entry.dedupe && !seen.insert(line.clone()) {
                continue;
            }

            // Replace ${1} placeholder with the line content
            let resolved_action = resolve_source_placeholders(&entry.action, &line);

            // Execute the resolved action
            match &resolved_action {
                ruler::types::ActionType::SendKeys(keys) => {
                    if let Some(agent) = agent {
                        for key in keys {
                            agent.send_keys(key).await?;
                        }
                    }
                }
                ruler::types::ActionType::Workflow(workflow_name, args) => {
                    println!("🔄 Workflow: {} {:?}", workflow_name, args);
                    // TODO: Implement custom workflow execution if needed
                }
            }
            processed += 1;
        }

        if entry.dedupe {
            println!("✅ Processed {} unique items", processed);
        } else {
            println!("✅ Processed {} items", processed);
        }
    } else {
        // No source, just execute the action directly
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
        }
    }
    Ok(())
}

/// Resolve ${1} placeholders in action with source line content
fn resolve_source_placeholders(
    action: &ruler::types::ActionType,
    value: &str,
) -> ruler::types::ActionType {
    match action {
        ruler::types::ActionType::SendKeys(keys) => {
            let resolved_keys = keys.iter().map(|key| key.replace("${1}", value)).collect();
            ruler::types::ActionType::SendKeys(resolved_keys)
        }
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            let resolved_workflow = workflow_name.replace("${1}", value);
            let resolved_args = args.iter().map(|arg| arg.replace("${1}", value)).collect();
            ruler::types::ActionType::Workflow(resolved_workflow, resolved_args)
        }
    }
}

/// Execute an entry action using the appropriate mechanism
pub async fn execute_entry_action(
    agent: &agent::Agent,
    entry: &ruler::entry::CompiledEntry,
    _queue_manager: &SharedQueueManager,
) -> Result<()> {
    // DETAILED DEBUG LOGGING FOR ENTRY EXECUTION
    tracing::debug!("=== EXECUTING ENTRY ACTION ===");
    tracing::debug!("Entry name: {}", entry.name);
    tracing::debug!("Action type: {:?}", entry.action);

    // If there's a source command, execute it first and process its output
    if let Some(source) = &entry.source {
        println!("📦 Executing entry '{}' → Source: {}", entry.name, source);

        // Execute the source command
        let output = Command::new("sh").arg("-c").arg(source).output()?;

        if !output.status.success() {
            eprintln!(
                "❌ Source command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.to_string())
            .collect();

        println!("✅ Source command produced {} lines", lines.len());

        // Process each line with deduplication if needed
        let mut seen = HashSet::new();
        let mut processed = 0;

        for line in lines {
            // Skip if deduplication is enabled and we've seen this line
            if entry.dedupe && !seen.insert(line.clone()) {
                continue;
            }

            // Replace ${1} placeholder with the line content
            let resolved_action = resolve_source_placeholders(&entry.action, &line);

            // Execute the resolved action
            match &resolved_action {
                ruler::types::ActionType::SendKeys(keys) => {
                    // Start command tracking before sending keys
                    if let Ok(shell_pid) = agent.get_shell_pid().await {
                        agent.start_command_tracking(shell_pid).await;
                    }

                    for key in keys {
                        tracing::debug!("📤 Sending individual key: {:?}", key);

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
                    println!("🔄 Workflow: {} {:?}", workflow_name, args);
                    // TODO: Implement custom workflow execution if needed
                }
            }
            processed += 1;
        }

        if entry.dedupe {
            println!("✅ Processed {} unique items", processed);
        } else {
            println!("✅ Processed {} items", processed);
        }
    } else {
        // No source, just execute the action directly
        match &entry.action {
            ruler::types::ActionType::SendKeys(keys) => {
                println!("🤖 Executing entry '{}' → Sending: {:?}", entry.name, keys);

                tracing::debug!("SendKeys action with {} keys:", keys.len());
                for (i, key) in keys.iter().enumerate() {
                    tracing::debug!("  Key {}: {:?}", i, key);
                }

                // Start command tracking before sending keys
                if let Ok(shell_pid) = agent.get_shell_pid().await {
                    agent.start_command_tracking(shell_pid).await;
                }

                for key in keys {
                    tracing::debug!("📤 Sending individual key: {:?}", key);

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
        }
    }
    Ok(())
}

/// Execute a rule action
pub async fn execute_rule_action(
    action: &ruler::types::ActionType,
    agent: &agent::Agent,
    _queue_manager: &SharedQueueManager,
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

                // Start command tracking before sending keys for rule actions too
                if let Ok(shell_pid) = agent.get_shell_pid().await {
                    agent.start_command_tracking(shell_pid).await;
                }

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
    }
    Ok(())
}

/// Process PTY output for pattern matching
pub async fn process_pty_output(
    pty_output: &str,
    agent: &agent::Agent,
    ruler: &ruler::Ruler,
    queue_manager: &SharedQueueManager,
) -> Result<()> {
    // Remove ANSI escape sequences for cleaner pattern matching
    let clean_output = strip_ansi_escapes(pty_output);

    tracing::debug!("=== PTY OUTPUT ===");
    tracing::debug!("Raw output: {:?}", pty_output);
    tracing::debug!("Clean output: {:?}", clean_output);
    tracing::debug!("==> Will check rules for PTY output");

    // Split by both \n and \r for better handling of carriage returns
    let lines: Vec<&str> = clean_output
        .split(['\n', '\r'])
        .filter(|line| !line.trim().is_empty())
        .collect();

    // Check each line for pattern matching and timeout rules
    for line in lines {
        tracing::debug!("Checking line: {:?}", line);

        let actions = ruler.decide_actions_with_timeout(line).await;

        tracing::debug!("Actions decided: {:?}", actions);

        for action in actions {
            execute_rule_action(&action, agent, queue_manager).await?;
        }
    }

    Ok(())
}

/// Strip ANSI escape sequences from text
fn strip_ansi_escapes(text: &str) -> String {
    let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap();
    ansi_regex.replace_all(text, "").to_string()
}
