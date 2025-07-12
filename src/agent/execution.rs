use crate::agent::Agent;
use crate::config;
use anyhow::Result;
use std::process::Command;
use tokio::time::Duration;

// ================== Execution Functions ==================

/// Execute an entry action (unified for all entry types)
pub async fn execute_entry(entry: &config::trigger::Trigger, agent: &Agent) -> Result<()> {
    tracing::info!("📦 Executing entry '{}': {:?}", entry.name, entry.action);

    if let Some(source) = &entry.source {
        execute_source_command(entry, source, agent).await
    } else {
        execute_action(&entry.action, agent, &format!("Entry '{}'", entry.name)).await
    }
}

/// Execute a source command and process its output
async fn execute_source_command(
    entry: &config::trigger::Trigger,
    source: &str,
    agent: &Agent,
) -> Result<()> {
    tracing::debug!("Executing source command: {}", source);
    let output = Command::new("sh").arg("-c").arg(source).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Source command failed: {} (stderr: {})",
            source,
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    if lines.is_empty() {
        tracing::info!("Source command '{}' produced no output", source);
        return Ok(());
    }

    tracing::info!("Source command '{}' produced {} lines", source, lines.len());

    // Process each line from the source command
    for (i, line) in lines.iter().enumerate() {
        let resolved_action = resolve_placeholders(&entry.action, line);
        let context = format!("Source line {}/{}", i + 1, lines.len());

        tracing::debug!(
            "{}: {}",
            context,
            line.chars().take(100).collect::<String>()
        );

        if let Err(e) = execute_action(&resolved_action, agent, &context).await {
            tracing::error!("Failed to process {}: {}", context, e);
        }

        // Small delay between lines to prevent overwhelming the system
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

/// Execute an action with consistent 100ms delay between keys
pub async fn execute_action(
    action: &config::types::ActionType,
    agent: &Agent,
    context: &str,
) -> Result<()> {
    let config::types::ActionType::SendKeys(keys) = action;
    if keys.is_empty() {
        tracing::debug!("{}: No keys to send", context);
        return Ok(());
    }

    tracing::info!("{}: Sending {} keys", context, keys.len());
    tracing::debug!("{}: Keys: {:?}", context, keys);

    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        agent.send_keys(key).await?;
    }

    Ok(())
}

/// Resolve ${1} placeholders in action with source line content
fn resolve_placeholders(
    action: &config::types::ActionType,
    line: &str,
) -> config::types::ActionType {
    match action {
        config::types::ActionType::SendKeys(keys) => {
            let resolved_keys = keys.iter().map(|key| key.replace("${1}", line)).collect();
            config::types::ActionType::SendKeys(resolved_keys)
        }
    }
}
