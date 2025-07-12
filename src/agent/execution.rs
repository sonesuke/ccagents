use crate::agent::Agent;
use crate::config;
use anyhow::Result;
use std::collections::HashSet;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Execute a periodic entry action (with agent context)
pub async fn execute_periodic_entry(
    entry: &config::entry::CompiledEntry,
    agent: Option<&Agent>,
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
            let resolved_action =
                crate::config::placeholder::resolve_source_placeholders(&entry.action, &line);

            // Execute the resolved action
            match &resolved_action {
                config::types::ActionType::SendKeys(keys) => {
                    if let Some(agent) = agent {
                        for key in keys {
                            agent.send_keys(key).await?;
                        }
                    }
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
            config::types::ActionType::SendKeys(keys) => {
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
        }
    }
    Ok(())
}

/// Execute an entry action using the appropriate mechanism
pub async fn execute_entry_action(
    agent: &Agent,
    entry: &config::entry::CompiledEntry,
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
            let resolved_action =
                crate::config::placeholder::resolve_source_placeholders(&entry.action, &line);

            // Execute the resolved action
            match &resolved_action {
                config::types::ActionType::SendKeys(keys) => {
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
            config::types::ActionType::SendKeys(keys) => {
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
        }
    }
    Ok(())
}

/// Execute a rule action
pub async fn execute_rule_action(action: &config::types::ActionType, agent: &Agent) -> Result<()> {
    match action {
        config::types::ActionType::SendKeys(keys) => {
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
    }
    Ok(())
}
