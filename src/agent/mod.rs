pub mod pty_process;
pub mod pty_session;
pub mod pty_terminal;

use crate::agent::pty_process::{PtyProcess, PtyProcessConfig};
use crate::config::app_config::MonitorConfig;
use crate::web_server::WebServer;
use anyhow::Result;
use std::process::Command;
use std::sync::{Arc, RwLock};
use tokio::task::JoinHandle;

/// Agent status for state management
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,   // Waiting and monitoring triggers
    Active, // Executing tasks and monitoring rules
}

/// Agent pool for managing multiple agents in parallel
pub struct AgentPool {
    agents: Vec<Arc<Agent>>,
}

impl AgentPool {
    /// Create a new agent pool from monitor configuration
    pub async fn new(monitor_config: &MonitorConfig) -> Result<Self> {
        let mut agents = Vec::new();
        let pool_size = monitor_config.get_agent_pool_size();
        let base_port = monitor_config.get_web_ui_port();
        let test_mode = crate::config::is_test_mode();

        for i in 0..pool_size {
            let port = base_port + i as u16;
            let agent_id = format!("agent-{}", i);
            let (cols, rows) = monitor_config.get_agent_dimensions(i);
            let agent = Arc::new(
                Agent::new(
                    agent_id,
                    test_mode,
                    port,
                    cols,
                    rows,
                    monitor_config.web_ui.host.clone(),
                    monitor_config.web_ui.enabled,
                )
                .await?,
            );

            // Start web server if enabled
            if monitor_config.web_ui.enabled {
                agent
                    .clone()
                    .start_web_server(port, monitor_config.web_ui.host.clone())
                    .await?;
            }

            agents.push(agent);
        }

        Ok(Self { agents })
    }

    /// Get the number of agents in the pool
    pub fn size(&self) -> usize {
        self.agents.len()
    }

    /// Get agent by index for web server assignment
    pub fn get_agent_by_index(&self, index: usize) -> Arc<Agent> {
        Arc::clone(&self.agents[index % self.agents.len()])
    }
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
