pub mod ht_process;
pub mod terminal_monitor;

use crate::agent::ht_process::{HtProcess, HtProcessConfig};
use crate::agent::terminal_monitor::{TerminalOutputMonitor, TerminalSnapshot};
use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

#[allow(dead_code)]
pub struct Agent {
    #[allow(dead_code)]
    id: String,
    ht_process: HtProcess,
    #[allow(dead_code)]
    terminal_monitor: Option<TerminalOutputMonitor>,
}

impl Agent {
    pub async fn new(id: String, test_mode: bool, port: u16) -> Result<Self> {
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

        Ok(Agent {
            id,
            ht_process,
            terminal_monitor: None,
        })
    }

    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        &self.id
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

    #[allow(dead_code)]
    pub async fn execute_command(&self, command: &str) -> Result<CommandResult> {
        info!("Agent {} executing command: {}", self.id, command);

        // Send command
        self.ht_process.send_input(format!("{}\n", command)).await?;

        // Wait for command to complete (simplified - in production would need better detection)
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Get output
        let output = self.ht_process.get_view().await?;

        // Try to parse exit code from output (simplified)
        let exit_code = if output.contains("command not found") {
            Some(127)
        } else {
            Some(0)
        };

        Ok(CommandResult {
            exit_code,
            output: output.clone(),
            error: String::new(),
            snapshot: Some(self.take_snapshot().await?),
        })
    }

    #[allow(dead_code)]
    pub async fn take_snapshot(&self) -> Result<TerminalSnapshot> {
        let content = self.ht_process.get_view().await?;

        // Get terminal size (simplified - would query actual size in production)
        let (width, height) = (80, 24);

        Ok(TerminalSnapshot {
            content,
            cursor_position: None, // Would parse from HT in production
            width,
            height,
        })
    }

    #[allow(dead_code)]
    pub async fn resize(&self, width: u32, height: u32) -> Result<()> {
        // HT process would handle resize in production
        info!("Agent {} resizing to {}x{}", self.id, width, height);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn is_available(&self) -> bool {
        self.ht_process.is_running().await
    }

    #[allow(dead_code)]
    pub fn backend_type(&self) -> &'static str {
        "ht"
    }

    #[allow(dead_code)]
    pub async fn get_environment(&self) -> Result<HashMap<String, String>> {
        // Send env command and parse output
        self.send_keys("env\n").await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let output = self.get_output().await?;
        let mut env_vars = HashMap::new();

        // Parse environment variables from output
        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                env_vars.insert(key.to_string(), value.to_string());
            }
        }

        Ok(env_vars)
    }

    #[allow(dead_code)]
    pub async fn set_working_directory(&self, path: &str) -> Result<()> {
        let cmd = format!("cd {}\n", path);
        self.send_keys(&cmd).await
    }

    #[allow(dead_code)]
    pub async fn start_monitoring(&mut self) -> Result<()> {
        if self.terminal_monitor.is_none() {
            let monitor = TerminalOutputMonitor::new(self.id.clone());
            self.terminal_monitor = Some(monitor);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn stop_monitoring(&mut self) {
        self.terminal_monitor = None;
    }

    #[allow(dead_code)]
    pub fn get_monitor(&self) -> Option<&TerminalOutputMonitor> {
        self.terminal_monitor.as_ref()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub output: String,
    pub error: String,
    pub snapshot: Option<TerminalSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let agent = Agent::new("test-agent".to_string(), true, 9999)
            .await
            .unwrap();
        assert_eq!(agent.id(), "test-agent");
        assert_eq!(agent.backend_type(), "ht");
    }

    #[tokio::test]
    async fn test_agent_availability() {
        let agent = Agent::new("test-agent".to_string(), true, 9999)
            .await
            .unwrap();
        // In test mode, agent may not be available since we don't start the process
        // This test mainly verifies that the agent can be created without panicking
        let _available = agent.is_available().await;
        // We just verify the call doesn't panic rather than checking the result
    }
}
