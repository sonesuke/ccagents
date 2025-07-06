pub mod pty_process;
pub mod pty_session;
pub mod pty_terminal;

use crate::agent::pty_process::{PtyProcess, PtyProcessConfig};
use crate::ruler::config::MonitorConfig;
use anyhow::Result;
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

/// Agent status for state management
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,   // Waiting and monitoring triggers
    Active, // Executing tasks and monitoring rules
}

/// Agent pool for managing multiple agents in parallel
pub struct AgentPool {
    agents: Vec<Arc<Agent>>,
    #[allow(dead_code)]
    next_index: AtomicUsize,
}

impl AgentPool {
    /// Create a new agent pool with the specified size
    pub async fn new(
        pool_size: usize,
        base_port: u16,
        test_mode: bool,
        monitor_config: &MonitorConfig,
    ) -> Result<Self> {
        let mut agents = Vec::new();

        for i in 0..pool_size {
            let port = base_port + i as u16;
            let agent_id = format!("agent-{}", i);
            let (cols, rows) = monitor_config.get_agent_dimensions(i);
            let agent = Arc::new(Agent::new(agent_id, test_mode, port, cols, rows).await?);
            agents.push(agent);
        }

        Ok(Self {
            agents,
            next_index: AtomicUsize::new(0),
        })
    }

    /// Get the next agent using round-robin selection
    #[allow(dead_code)]
    pub fn get_agent(&self) -> Arc<Agent> {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed) % self.agents.len();
        Arc::clone(&self.agents[index])
    }

    /// Get the number of agents in the pool
    pub fn size(&self) -> usize {
        self.agents.len()
    }

    /// Get agent by index for web server assignment
    pub fn get_agent_by_index(&self, index: usize) -> Arc<Agent> {
        Arc::clone(&self.agents[index % self.agents.len()])
    }

    /// Get an idle agent (first available)
    pub async fn get_idle_agent(&self) -> Option<Arc<Agent>> {
        for agent in &self.agents {
            if agent.get_status().await == AgentStatus::Idle {
                return Some(Arc::clone(agent));
            }
        }
        None
    }
}

pub struct Agent {
    id: String,
    ht_process: PtyProcess,
    cols: u16,
    rows: u16,
    status: RwLock<AgentStatus>,
    command_start_time: RwLock<Option<std::time::Instant>>,
}

impl Agent {
    pub async fn new(
        id: String,
        test_mode: bool,
        _port: u16,
        cols: u16,
        rows: u16,
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
    #[allow(dead_code)]
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
    pub async fn monitor_command_completion(&self) {
        // Get the shell PID
        if let Ok(Some(shell_pid)) = self.get_shell_pid().await {
            // Get current child processes of the shell
            let child_pids = get_child_processes(shell_pid);

            if child_pids.is_empty() {
                // No child processes = shell is at prompt = command completed
                tracing::info!(
                    "💀 Agent {} detected command completion (no child processes)",
                    self.id
                );
                self.stop_command_tracking().await;
                self.set_status(AgentStatus::Idle).await;
            } else {
                // Child processes still running = command still active
                tracing::debug!(
                    "🔄 Agent {} has active child processes: {:?}",
                    self.id,
                    child_pids
                );
            }
        } else {
            tracing::debug!("❌ Agent {} could not get shell PID", self.id);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let _agent = Agent::new("test-agent".to_string(), true, 9999, 80, 24)
            .await
            .unwrap();
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }
}
