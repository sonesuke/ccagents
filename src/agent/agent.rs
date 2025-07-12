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

    /// Monitor command completion by checking child processes of the shell
    pub async fn monitor_command_completion(&self) {
        if let Ok(Some(shell_pid)) = self.process.get_shell_pid().await {
            let child_pids = get_child_processes(shell_pid);
            let current_status = self.get_status().await;

            match (!child_pids.is_empty(), current_status) {
                (true, AgentStatus::Idle) => {
                    self.set_status(AgentStatus::Active).await;
                    tracing::debug!("🔄 Agent {} → Active", self.get_id());
                }
                (false, AgentStatus::Active) => {
                    self.set_status(AgentStatus::Idle).await;
                    tracing::debug!("✅ Agent {} → Idle", self.get_id());
                }
                _ => {}
            }
        }
    }

    /// Start monitoring this agent's status (Active/Idle) based on child processes
    pub async fn start_status_monitoring(self: std::sync::Arc<Self>) -> Result<()> {
        loop {
            self.monitor_command_completion().await;
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
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect(),
        _ => Vec::new(),
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
