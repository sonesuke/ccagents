pub mod pty_process;
pub mod pty_session;
pub mod pty_terminal;

use crate::agent::pty_process::{PtyProcess, PtyProcessConfig};
use crate::ruler::config::MonitorConfig;
use anyhow::Result;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

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
}

pub struct Agent {
    ht_process: PtyProcess,
    #[allow(dead_code)]
    cols: u16,
    #[allow(dead_code)]
    rows: u16,
}

impl Agent {
    pub async fn new(
        _id: String,
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
            ht_process,
            cols,
            rows,
        })
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        // AGENT SEND_KEYS DEBUG
        crate::debug_print!("🔄 Agent::send_keys called with: {:?}", keys);

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("pty_debug.log")
        {
            use std::io::Write;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let _ = writeln!(file, "[{}] === AGENT SEND_KEYS ===", timestamp);
            let _ = writeln!(file, "Keys: {:?}", keys);
            let _ = writeln!(file, "About to call ht_process.send_input");
            let _ = writeln!(file, "---");
        }

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

    /// Get raw ANSI output for asciinema player integration
    #[allow(dead_code)]
    pub async fn get_raw_ansi_output(&self) -> Result<Option<String>> {
        self.ht_process
            .get_raw_ansi_output()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get terminal output for WebSocket integration
    #[allow(dead_code)]
    pub async fn get_terminal_output(&self) -> Result<String> {
        Ok(self.ht_process.get_avt_terminal_output().await)
    }

    /// Get terminal dimensions for asciinema integration
    #[allow(dead_code)]
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
