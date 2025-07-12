use crate::config::terminal::TerminalConfig;
use crate::terminal::pty_process::{PtyProcess, PtyProcessConfig};
use crate::web_server::WebServer;
use anyhow::Result;
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
    process: PtyProcess,
    terminal_config: TerminalConfig,
    status: RwLock<AgentStatus>,
    web_server_handle: RwLock<Option<JoinHandle<()>>>,
}

impl Agent {
    pub async fn new(id: String, terminal_config: TerminalConfig) -> Result<Self> {
        let config = PtyProcessConfig {
            shell_command: Some(terminal_config.shell_command.clone()),
            cols: terminal_config.cols,
            rows: terminal_config.rows,
        };

        let process = PtyProcess::new(config);

        // Start the PTY process
        process.start().await?;

        Ok(Agent {
            id,
            process,
            terminal_config,
            status: RwLock::new(AgentStatus::Idle),
            web_server_handle: RwLock::new(None),
        })
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        self.get_process()
            .send_input(keys.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send keys: {}", e))
    }

    /// Get terminal dimensions for asciinema integration
    pub fn get_terminal_config(&self) -> &TerminalConfig {
        &self.terminal_config
    }

    /// Get agent ID
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Get access to the PTY process
    pub fn get_process(&self) -> &PtyProcess {
        &self.process
    }

    /// Check if the agent is currently active (true = Active, false = Idle)
    pub async fn is_active(&self) -> bool {
        matches!(*self.status.read().unwrap(), AgentStatus::Active)
    }

    /// Set the status of the agent
    async fn set_status(&self, new_status: AgentStatus) {
        let mut status = self.status.write().unwrap();
        let old_status = status.clone();

        if old_status != new_status {
            *status = new_status.clone();
            tracing::debug!("🔄 Agent {} → {:?}", self.get_id(), new_status);
        }
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
                tracing::error!("❌ Web server failed on port {}: {}", port, e);
            }
        });

        *self.web_server_handle.write().unwrap() = Some(handle);
        Ok(())
    }

    /// Monitor agent status by checking child processes
    async fn monitor(&self) {
        if let Ok(child_pids) = self.get_process().get_child_processes().await {
            let new_status = if child_pids.is_empty() {
                AgentStatus::Idle
            } else {
                AgentStatus::Active
            };

            self.set_status(new_status).await;
        }
    }

    /// Start monitoring this agent's status
    pub async fn start_monitoring(self: std::sync::Arc<Self>) -> Result<()> {
        loop {
            self.monitor().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let terminal_config = TerminalConfig::new(80, 24);
        let _agent = Agent::new("test-agent".to_string(), terminal_config)
            .await
            .unwrap();
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }
}
