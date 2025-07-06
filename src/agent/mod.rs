pub mod pty_process;
pub mod pty_session;
pub mod pty_terminal;

use crate::agent::pty_process::{PtyProcess, PtyProcessConfig};
use crate::ruler::config::MonitorConfig;
use anyhow::Result;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

/// Agent status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,   // Waiting and monitoring triggers
    Active, // Executing tasks and monitoring rules
}

/// Agent pool for managing multiple agents in parallel
pub struct AgentPool {
    agents: Vec<Arc<Agent>>,
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

    /// Get an idle agent from the pool
    pub async fn get_idle_agent(&self) -> Option<Arc<Agent>> {
        for agent in &self.agents {
            if agent.is_idle().await {
                return Some(Arc::clone(agent));
            }
        }
        None
    }

    /// Get all active agents
    pub async fn get_active_agents(&self) -> Vec<Arc<Agent>> {
        let mut active_agents = Vec::new();
        for agent in &self.agents {
            if agent.is_active().await {
                active_agents.push(Arc::clone(agent));
            }
        }
        active_agents
    }

    /// Get status of all agents
    #[allow(dead_code)]
    pub async fn get_agent_statuses(&self) -> Vec<(String, AgentStatus)> {
        let mut statuses = Vec::new();
        for agent in &self.agents {
            let status = agent.get_status().await;
            statuses.push((agent.get_id().to_string(), status));
        }
        statuses
    }
}

pub struct Agent {
    id: String,
    status: Arc<RwLock<AgentStatus>>,
    ht_process: PtyProcess,
    cols: u16,
    rows: u16,
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
            status: Arc::new(RwLock::new(AgentStatus::Idle)),
            ht_process,
            cols,
            rows,
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

    /// Get command's direct output (stdout/stderr)
    pub async fn get_command_output(&self) -> Option<crate::agent::pty_process::CommandOutput> {
        self.ht_process.get_command_output().await
    }

    /// Send input to the terminal (for WebSocket)
    pub async fn send_input(&self, input: &str) -> Result<()> {
        self.send_keys(input).await
    }

    /// Get accumulated terminal output for initial WebSocket state
    pub async fn get_accumulated_output(&self) -> Result<String> {
        let bytes = self.ht_process.get_accumulated_output().await;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Get terminal dimensions for asciinema integration
    pub fn get_terminal_size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Get direct access to PTY output broadcast receiver for WebSocket streaming
    pub async fn get_pty_output_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<String>> {
        self.ht_process
            .get_pty_output_receiver()
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
        *self.status.write().unwrap() = new_status.clone();
        tracing::debug!("🔄 Agent {} status changed to {:?}", self.id, new_status);
    }

    /// Check if the agent is idle
    pub async fn is_idle(&self) -> bool {
        *self.status.read().unwrap() == AgentStatus::Idle
    }

    /// Check if the agent is active
    pub async fn is_active(&self) -> bool {
        *self.status.read().unwrap() == AgentStatus::Active
    }

    /// Get agent ID
    pub fn get_id(&self) -> &str {
        &self.id
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
