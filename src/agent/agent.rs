use crate::config;
use crate::terminal::pty_process::{PtyProcess, PtyProcessConfig};
use crate::web_server::WebServer;
use anyhow::Result;
use std::collections::HashSet;
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
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
    ht_process: PtyProcess,
    cols: u16,
    rows: u16,
    status: RwLock<AgentStatus>,
    command_start_time: RwLock<Option<std::time::Instant>>,
    #[allow(dead_code)] // Will be used for future WebServer lifecycle management
    web_server: Option<WebServer>,
    web_server_handle: RwLock<Option<JoinHandle<()>>>,
}

impl Agent {
    pub async fn new(
        id: String,
        test_mode: bool,
        _port: u16,
        cols: u16,
        rows: u16,
        _host: String,
        _web_ui_enabled: bool,
    ) -> Result<Self> {
        let config = PtyProcessConfig {
            shell_command: Some(std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())),
            cols,
            rows,
        };

        let ht_process = PtyProcess::new(config);

        // Start the HT process
        if !test_mode {
            ht_process.start().await?;
        }

        Ok(Agent {
            id,
            ht_process,
            cols,
            rows,
            status: RwLock::new(AgentStatus::Idle),
            command_start_time: RwLock::new(None),
            web_server: None, // Will be set after creation
            web_server_handle: RwLock::new(None),
        })
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        // AGENT SEND_KEYS DEBUG
        tracing::debug!("🔄 Agent::send_keys called with: {:?}", keys);

        tracing::debug!("=== AGENT SEND_KEYS ===");
        tracing::debug!("Keys: {:?}", keys);
        tracing::debug!("About to call ht_process.send_input");

        self.ht_process
            .send_input(keys.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send keys: {}", e))
    }

    /// Send input to the terminal (for WebSocket)
    pub async fn send_input(&self, input: &str) -> Result<()> {
        self.send_keys(input).await
    }

    /// Get terminal dimensions for asciinema integration
    pub fn get_terminal_size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Get direct access to PTY raw bytes receiver for WebSocket streaming
    pub async fn get_pty_bytes_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<bytes::Bytes>> {
        self.ht_process
            .get_pty_bytes_receiver()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get direct access to PTY string receiver for rule matching
    pub async fn get_pty_string_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<String>> {
        self.ht_process
            .get_pty_string_receiver()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get current screen contents from vt100::Parser for WebSocket initial state
    pub async fn get_screen_contents(&self) -> Result<String> {
        self.ht_process
            .get_screen_contents()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get agent ID
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Get the shell PID
    pub async fn get_shell_pid(&self) -> Result<Option<u32>> {
        self.ht_process
            .get_shell_pid()
            .await
            .map_err(|e| anyhow::anyhow!(e))
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

    /// Start tracking a command execution (using shell monitoring)
    pub async fn start_command_tracking(&self, _shell_pid: Option<u32>) {
        // We now monitor child processes instead of specific PIDs
        if let Ok(mut start_time) = self.command_start_time.write() {
            *start_time = Some(std::time::Instant::now());
        }
        tracing::info!(
            "🚀 Agent {} started command tracking (child process monitoring)",
            self.id
        );
    }

    /// Stop tracking command execution
    pub async fn stop_command_tracking(&self) {
        if let Ok(mut start_time) = self.command_start_time.write() {
            *start_time = None;
        }
        tracing::info!("🏁 Agent {} stopped command tracking", self.id);
    }

    /// Monitor command completion by checking child processes of the shell
    /// Automatically manages Active/Idle status based on child process presence
    pub async fn monitor_command_completion(&self) {
        // Get the shell PID
        if let Ok(Some(shell_pid)) = self.get_shell_pid().await {
            // Get current child processes of the shell
            let child_pids = get_child_processes(shell_pid);
            let current_status = self.get_status().await;

            if !child_pids.is_empty() && current_status == AgentStatus::Idle {
                // Child processes present + Idle → switch to Active
                tracing::info!(
                    "🚀 Agent {} detected child processes, setting to Active: {:?}",
                    self.id,
                    child_pids
                );
                self.set_status(AgentStatus::Active).await;
            } else if child_pids.is_empty() && current_status == AgentStatus::Active {
                // No child processes + Active → switch to Idle
                tracing::info!(
                    "💀 Agent {} detected command completion (no child processes), setting to Idle",
                    self.id
                );
                self.stop_command_tracking().await;
                self.set_status(AgentStatus::Idle).await;
            } else {
                // Status matches child process state, no change needed
                tracing::trace!(
                    "✅ Agent {} status consistent: {:?} (child processes: {:?})",
                    self.id,
                    current_status,
                    child_pids
                );
            }
        } else {
            tracing::debug!("❌ Agent {} could not get shell PID", self.id);
        }
    }

    /// Start the WebServer for this agent if configured
    pub async fn start_web_server(self: Arc<Self>, port: u16, host: String) -> Result<()> {
        let web_server = WebServer::new(port, host, Arc::clone(&self));
        let handle = tokio::spawn(async move {
            if let Err(e) = web_server.start().await {
                eprintln!("❌ Web server failed on port {}: {}", port, e);
            }
        });

        *self.web_server_handle.write().unwrap() = Some(handle);
        Ok(())
    }

    /// Stop the WebServer for this agent
    #[allow(dead_code)] // Will be used for graceful shutdown
    pub fn stop_web_server(&self) {
        if let Some(handle) = self.web_server_handle.write().unwrap().take() {
            handle.abort();
        }
    }

    /// Check if this agent has a running web server
    #[allow(dead_code)] // Will be used for status monitoring
    pub fn has_web_server(&self) -> bool {
        self.web_server_handle.read().unwrap().is_some()
    }

    /// Start monitoring this agent's status (Active/Idle) based on child processes
    pub async fn start_status_monitoring(self: Arc<Self>) -> Result<()> {
        loop {
            // Monitor command completion to auto-manage Active/Idle status
            self.monitor_command_completion().await;

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Get child processes of a given parent PID
fn get_child_processes(parent_pid: u32) -> Vec<u32> {
    let output = Command::new("pgrep")
        .arg("-P")
        .arg(parent_pid.to_string())
        .output();

    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect(),
        Err(e) => {
            tracing::debug!(
                "Failed to get child processes for PID {}: {}",
                parent_pid,
                e
            );
            Vec::new()
        }
    }
}

// ================== Execution Functions ==================

/// Execute an entry action (unified for all entry types)
pub async fn execute_entry(entry: &config::trigger::CompiledEntry, agent: &Agent) -> Result<()> {
    tracing::debug!("Entry name: {}", entry.name);
    tracing::debug!("Action type: {:?}", entry.action);

    if let Some(source) = &entry.source {
        println!("📦 Executing entry '{}' → Source: {}", entry.name, source);
        execute_source_command(entry, source, Some(agent)).await
    } else {
        execute_action_with_context(&entry.action, &entry.name, Some(agent)).await
    }
}

/// Execute a rule action
pub async fn execute_rule_action(action: &config::types::ActionType, agent: &Agent) -> Result<()> {
    execute_send_keys_action(action, Some(agent), "🤖 EXECUTING RULE", Some(1000)).await
}

/// Execute a source command and process its output
async fn execute_source_command(
    entry: &config::trigger::CompiledEntry,
    source: &str,
    agent: Option<&Agent>,
) -> Result<()> {
    let output = Command::new("sh").arg("-c").arg(source).output()?;

    if !output.status.success() {
        eprintln!(
            "❌ Source command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(());
    }

    let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    println!("✅ Source command produced {} lines", lines.len());

    let mut seen = HashSet::new();
    let mut processed = 0;

    for line in lines {
        if entry.dedupe && !seen.insert(line.clone()) {
            continue;
        }

        let resolved_action = resolve_source_placeholders(&entry.action, &line);
        execute_action(&resolved_action, agent).await?;
        processed += 1;
    }

    let suffix = if entry.dedupe {
        "unique items"
    } else {
        "items"
    };
    println!("✅ Processed {} {}", processed, suffix);
    Ok(())
}

/// Execute an action with optional context information
async fn execute_action_with_context(
    action: &config::types::ActionType,
    entry_name: &str,
    agent: Option<&Agent>,
) -> Result<()> {
    execute_action_internal(action, agent, Some(entry_name)).await
}

/// Execute an action (common logic for both source and direct actions)
async fn execute_action(action: &config::types::ActionType, agent: Option<&Agent>) -> Result<()> {
    execute_action_internal(action, agent, None).await
}

/// Internal action execution with optional context
async fn execute_action_internal(
    action: &config::types::ActionType,
    agent: Option<&Agent>,
    entry_name: Option<&str>,
) -> Result<()> {
    match agent {
        Some(agent) => {
            let prefix = entry_name
                .map(|name| format!("🤖 Executing entry '{}'", name))
                .unwrap_or_default();
            execute_send_keys_action(action, Some(agent), &prefix, None).await
        }
        None => {
            if let Some(name) = entry_name {
                println!(
                    "⚠️ Entry '{}' has SendKeys action - skipping (no agent context)",
                    name
                );
            }
            Ok(())
        }
    }
}

/// Common SendKeys execution logic with configurable output and delay
async fn execute_send_keys_action(
    action: &config::types::ActionType,
    agent: Option<&Agent>,
    log_prefix: &str,
    post_delay_ms: Option<u64>,
) -> Result<()> {
    match action {
        config::types::ActionType::SendKeys(keys) => {
            if let Some(agent) = agent {
                if !keys.is_empty() {
                    if !log_prefix.is_empty() {
                        println!("{} → Sending: {:?}", log_prefix, keys);
                        if log_prefix.contains("RULE") {
                            println!(
                                "🕐 Timestamp: {}",
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                            );
                        }
                    }

                    start_command_tracking(agent).await;
                    send_keys_with_delay(keys, agent, 100).await?;

                    if let Some(delay_ms) = post_delay_ms {
                        println!("✅ Rule execution completed, waiting {}ms", delay_ms);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        }
    }
}

/// Start command tracking for an agent
async fn start_command_tracking(agent: &Agent) {
    if let Ok(shell_pid) = agent.get_shell_pid().await {
        agent.start_command_tracking(shell_pid).await;
    }
}

/// Send keys with delay between each key
async fn send_keys_with_delay(keys: &[String], agent: &Agent, delay_ms: u64) -> Result<()> {
    for key in keys {
        tracing::debug!("📤 Sending individual key: {:?}", key);

        let key_to_send = if key == "\\r" || key == "\r" {
            "\r"
        } else {
            key
        };

        if let Err(e) = agent.send_keys(key_to_send).await {
            eprintln!("❌ Error sending key: {}", e);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }
    Ok(())
}

/// Resolve ${1} placeholders in action with source line content
fn resolve_source_placeholders(
    action: &config::types::ActionType,
    value: &str,
) -> config::types::ActionType {
    match action {
        config::types::ActionType::SendKeys(keys) => {
            let resolved_keys = keys.iter().map(|key| key.replace("${1}", value)).collect();
            config::types::ActionType::SendKeys(resolved_keys)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let _agent = Agent::new(
            "test-agent".to_string(),
            true,
            9999,
            80,
            24,
            "localhost".to_string(),
            false, // web_ui_enabled = false for test
        )
        .await
        .unwrap();
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }
}
