pub mod pty_process;
pub mod pty_session;
pub mod pty_terminal;

use crate::agent::pty_process::{PtyProcess, PtyProcessConfig};
use anyhow::Result;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tracing::info;

fn clean_terminal_output(raw_output: &str) -> String {
    if raw_output.is_empty() {
        return String::new();
    }

    let ansi_regex = regex::Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]|\x1B\[[\?]?[0-9;]*[hlm]|\x1B[>\=]|\x1B[c\d]|\x1B\][0-9];|\x1B\[[0-9A-Z]|\x1B[789]|\x1B\([AB]|\x1B\[[0-9]*[HJKfABCDGR`]|\x1B\[[0-9;]*[rW]|\x1B\[[0-9;]*H|\u{9b}[0-9;]*[a-zA-Z]|\u{9b}[\?]?[0-9;]*[hlm]").unwrap();
    let without_ansi = ansi_regex.replace_all(raw_output, "");

    let control_regex = regex::Regex::new(r"[\x00-\x1F\x7F\u{9b}]+").unwrap();
    let clean_text = control_regex.replace_all(&without_ansi, "");

    let lines: Vec<&str> = clean_text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    if lines.is_empty() {
        String::new()
    } else {
        lines.join(" ")
    }
}

/// Result of terminal differential detection
pub struct DifferentialContent {
    pub new_content: String,
    pub clean_content: String,
}

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
        self.ht_process
            .send_input(keys.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send keys: {}", e))
    }

    pub async fn get_output(&self) -> Result<String> {
        self.ht_process
            .get_view()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get output: {}", e))
    }

    /// Detect differential content from terminal output
    ///
    /// This function implements the HT TERMINAL DIFFERENTIAL DETECTION STRATEGY:
    /// - HT Terminal sends the entire screen buffer as a single continuous string
    /// - The buffer is fixed-width with space padding
    /// - We compare buffers character-by-character and extract only newly added content
    /// - This treats the terminal as an append-only stream
    pub fn detect_differential_content(
        &self,
        current_output: &str,
        previous_output: Option<&str>,
    ) -> DifferentialContent {
        let current_output = current_output.trim();
        let mut new_content = String::new();

        if let Some(previous_output) = previous_output {
            // Find the longest common prefix between previous and current output
            let common_prefix_len = previous_output
                .chars()
                .zip(current_output.chars())
                .take_while(|(a, b)| a == b)
                .count();

            // Extract the new content that was appended to the end
            // Handle Unicode character boundaries safely
            if current_output.len() > common_prefix_len {
                // Find a safe character boundary at or after common_prefix_len
                let safe_start = current_output
                    .char_indices()
                    .find(|(i, _)| *i >= common_prefix_len)
                    .map(|(i, _)| i)
                    .unwrap_or(current_output.len());

                if safe_start < current_output.len() {
                    new_content = current_output[safe_start..].trim().to_string();
                }
            }

            // Debug info (only shown with --debug flag)
            info!(
                "Buffer length: prev={}, curr={}",
                previous_output.len(),
                current_output.len()
            );
            info!("Common prefix length: {}", common_prefix_len);
        } else {
            // First time - entire output is "new"
            new_content = current_output.to_string();
        }

        // Clean the content (simple ANSI escape sequence removal)
        let clean_content = clean_terminal_output(&new_content);

        DifferentialContent {
            new_content,
            clean_content,
        }
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
