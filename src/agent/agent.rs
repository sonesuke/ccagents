use crate::config;
use crate::terminal::pty_process::{PtyProcess, PtyProcessConfig};
use crate::web_server::WebServer;
use anyhow::Result;
use std::process::Command;
use std::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Duration;

/// Agent status for state management
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,   // Waiting and monitoring triggers
    Active, // Executing tasks and monitoring rules
}

pub struct Agent {
    id: String,
    pub process: PtyProcess,
    cols: u16,
    rows: u16,
    status: RwLock<AgentStatus>,
    web_server_handle: RwLock<Option<JoinHandle<()>>>,
}

impl Agent {
    pub async fn new(id: String, test_mode: bool, cols: u16, rows: u16) -> Result<Self> {
        let config = PtyProcessConfig {
            shell_command: Some(std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())),
            cols,
            rows,
        };

        let process = PtyProcess::new(config);

        // Start the HT process
        if !test_mode {
            process.start().await?;
        }

        Ok(Agent {
            id,
            process,
            cols,
            rows,
            status: RwLock::new(AgentStatus::Idle),
            web_server_handle: RwLock::new(None),
        })
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        // AGENT SEND_KEYS DEBUG
        tracing::debug!("🔄 Agent::send_keys called with: {:?}", keys);

        tracing::debug!("=== AGENT SEND_KEYS ===");
        tracing::debug!("Keys: {:?}", keys);
        tracing::debug!("About to call process.send_input");

        self.process
            .send_input(keys.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send keys: {}", e))
    }

    /// Get terminal dimensions for asciinema integration
    pub fn get_terminal_size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Get agent ID
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Get the current status of the agent
    pub async fn get_status(&self) -> AgentStatus {
        self.status.read().unwrap().clone()
    }

    /// Set the status of the agent
    pub async fn set_status(&self, new_status: AgentStatus) {
        let mut status = self.status.write().unwrap();
        *status = new_status;
    }

    /// Start the WebServer for this agent if configured
    pub async fn start_web_server(
        self: std::sync::Arc<Self>,
        port: u16,
        host: String,
    ) -> Result<()> {
        let web_server = WebServer::new(port, host, std::sync::Arc::clone(&self));
        let handle = tokio::spawn(async move {
            if let Err(e) = web_server.start().await {
                eprintln!("❌ Web server failed on port {}: {}", port, e);
            }
        });

        *self.web_server_handle.write().unwrap() = Some(handle);
        Ok(())
    }
}

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

// ================== Monitoring Functions ==================

/// Get child processes of a given parent PID
pub fn get_child_processes(parent_pid: u32) -> Vec<u32> {
    let output = Command::new("pgrep")
        .arg("-P")
        .arg(parent_pid.to_string())
        .output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// Monitor command completion by checking child processes of the shell
pub async fn monitor_command_completion(agent: &Agent) {
    if let Ok(Some(shell_pid)) = agent.process.get_shell_pid().await {
        let child_pids = get_child_processes(shell_pid);
        let current_status = agent.get_status().await;

        match (!child_pids.is_empty(), current_status) {
            (true, AgentStatus::Idle) => {
                agent.set_status(AgentStatus::Active).await;
                tracing::debug!("🔄 Agent {} → Active", agent.get_id());
            }
            (false, AgentStatus::Active) => {
                agent.set_status(AgentStatus::Idle).await;
                tracing::debug!("✅ Agent {} → Idle", agent.get_id());
            }
            _ => {}
        }
    }
}

/// Start monitoring this agent's status (Active/Idle) based on child processes
pub async fn start_status_monitoring(agent: std::sync::Arc<Agent>) -> Result<()> {
    loop {
        monitor_command_completion(&agent).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let _agent = Agent::new("test-agent".to_string(), true, 80, 24)
            .await
            .unwrap();
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }
}
