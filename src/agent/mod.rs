pub mod ht_process;

use crate::agent::ht_process::{HtProcess, HtProcessConfig};
use anyhow::Result;
use tracing::info;

/// Result of terminal differential detection
pub struct DifferentialContent {
    pub new_content: String,
    pub clean_content: String,
}

pub struct Agent {
    ht_process: HtProcess,
}

impl Agent {
    pub async fn new(_id: String, test_mode: bool, port: u16) -> Result<Self> {
        let config = if test_mode {
            // Test configuration
            HtProcessConfig {
                ht_binary_path: "mock_ht".to_string(),
                shell_command: Some("bash".to_string()),
                restart_attempts: 1,
                restart_delay_ms: 100,
                port,
            }
        } else {
            // Production configuration
            HtProcessConfig {
                ht_binary_path: which::which("ht")
                    .map_err(|_| anyhow::anyhow!("ht binary not found in PATH"))?
                    .to_string_lossy()
                    .to_string(),
                shell_command: Some(
                    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()),
                ),
                restart_attempts: 3,
                restart_delay_ms: 1000,
                port,
            }
        };

        let ht_process = HtProcess::new(config);

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

        // Clean the content
        let clean_content = HtProcess::clean_terminal_output(&new_content);

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
