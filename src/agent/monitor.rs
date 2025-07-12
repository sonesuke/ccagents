use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::Duration;

use crate::agent::terminal::{Agent, AgentStatus};
use crate::config::RuleConfig;

use crate::rule::Monitor;

/// Agent monitor responsible for monitoring a single terminal's PTY output and processing rules
pub struct AgentMonitor {
    pub rule_config: RuleConfig,
    pub agent: Arc<Agent>,
    pub receiver: broadcast::Receiver<String>,
}

impl Monitor for AgentMonitor {
    async fn start_monitoring(self) -> Result<()> {
        let monitor = self;
        monitor.start_monitoring().await
    }
}

impl AgentMonitor {
    pub async fn start_monitoring(mut self) -> Result<()> {
        loop {
            // Monitor command completion to auto-manage Active/Idle status
            self.agent.monitor_command_completion().await;
            let current_status = self.agent.get_status().await;

            // Process PTY output
            self.receive_and_process_pty_output(current_status).await?;

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn receive_and_process_pty_output(&mut self, status: AgentStatus) -> Result<()> {
        let mut received_any = false;

        while let Ok(pty_output) = self.receiver.try_recv() {
            received_any = true;
            tracing::debug!(
                "📝 Agent {} ({:?}) received PTY output: {} bytes: '{}'",
                self.agent.get_id(),
                status,
                pty_output.len(),
                pty_output.chars().take(50).collect::<String>()
            );

            // Process rules only for Active agents
            if status == AgentStatus::Active {
                tracing::debug!(
                    "🔍 Processing rules for agent {} ({:?})",
                    self.agent.get_id(),
                    status
                );
                if let Err(e) = self.process_rules_for_output(&pty_output).await {
                    tracing::debug!("❌ Error processing PTY output: {}", e);
                }
            } else {
                tracing::trace!(
                    "⏸️  Skipping rule processing for agent {} (status: {:?})",
                    self.agent.get_id(),
                    status
                );
            }
        }

        if received_any {
            tracing::debug!("✅ Agent {} processed data chunks", self.agent.get_id());
        }

        Ok(())
    }

    /// Process rules for the given PTY output
    async fn process_rules_for_output(&self, pty_output: &str) -> Result<()> {
        // Reset timeout activity for diff_timeout rules whenever ANY terminal output is received
        // This ensures diff_timeout detects "no terminal output" rather than "no pattern matches"
        self.rule_config.reset_timeout_activity().await;

        // Remove ANSI escape sequences for cleaner pattern matching
        let clean_output = self.strip_ansi_escapes(pty_output);

        tracing::debug!("=== PTY OUTPUT ===");
        tracing::debug!("Raw output: {:?}", pty_output);
        tracing::debug!("Clean output: {:?}", clean_output);
        tracing::debug!("==> Will check rules for PTY output");

        // Split by both \n and \r for better handling of carriage returns
        let lines: Vec<&str> = clean_output
            .split(['\n', '\r'])
            .filter(|line| !line.trim().is_empty())
            .collect();

        // Check each line for pattern matching and timeout rules
        for line in lines {
            tracing::debug!("Checking line: {:?}", line);

            let actions = self.rule_config.decide_actions_with_timeout(line).await;

            tracing::debug!("Actions decided: {:?}", actions);

            for action in actions {
                crate::agent::execution::execute_rule_action(&action, &self.agent).await?;
            }
        }

        Ok(())
    }

    /// Strip ANSI escape sequences from text
    fn strip_ansi_escapes(&self, text: &str) -> String {
        let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap();
        ansi_regex.replace_all(text, "").to_string()
    }
}
