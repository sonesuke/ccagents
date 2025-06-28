pub mod pty_process;
pub mod pty_session;
pub mod pty_terminal;

use crate::agent::pty_process::{PtyProcess, PtyProcessConfig};
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
    pub async fn new(pool_size: usize, base_port: u16, test_mode: bool) -> Result<Self> {
        let mut agents = Vec::new();

        for i in 0..pool_size {
            let port = base_port + i as u16;
            let agent_id = format!("agent-{}", i);
            let agent = Arc::new(Agent::new(agent_id, test_mode, port).await?);
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
}

impl Agent {
    pub async fn new(_id: String, test_mode: bool, _port: u16) -> Result<Self> {
        let config = PtyProcessConfig {
            shell_command: Some(std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())),
            cols: 80,
            rows: 24,
        };

        let ht_process = PtyProcess::new(config);

        // Start the HT process
        if !test_mode {
            ht_process.start().await?;
        }

        Ok(Agent { ht_process })
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        // AGENT SEND_KEYS DEBUG
        println!("🔄 Agent::send_keys called with: {:?}", keys);

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

    /// Get current terminal output (for WebSocket)
    pub async fn get_terminal_output(&self) -> Result<String> {
        // Get the actual terminal screen content
        match self.ht_process.get_view().await {
            Ok(content) => {
                // Format content for better terminal display
                if content.is_empty() {
                    Ok("Terminal ready\r\n$ ".to_string())
                } else {
                    // Ensure proper line endings for terminal display
                    let formatted = content.replace('\n', "\r\n");
                    Ok(formatted)
                }
            }
            Err(_) => Ok("Terminal initializing...\r\n$ ".to_string()),
        }
    }

    /// Get terminal dimensions for asciinema integration
    #[allow(dead_code)] // Will be used in future enhancements
    pub fn get_terminal_size(&self) -> (u16, u16) {
        // Return cols, rows based on PTY configuration
        (80, 24) // Default size, can be made configurable later
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let _agent = Agent::new("test-agent".to_string(), true, 9999)
            .await
            .unwrap();
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }
}
