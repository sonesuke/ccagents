use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::agent::{Agent, AgentStatus};
use crate::config::RuleConfig;

/// Diff monitor responsible for processing PTY output and detecting pattern changes
pub struct DiffMonitor {
    pub rule_config: RuleConfig,
}

impl DiffMonitor {
    pub fn new(rule_config: RuleConfig) -> Self {
        Self { rule_config }
    }

    /// Process rules for the given PTY output
    pub async fn process_rules_for_output(&self, pty_output: &str, agent: &Agent) -> Result<()> {
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
                crate::agent::execution::execute_rule_action(&action, agent).await?;
            }
        }

        Ok(())
    }

    /// Start monitoring PTY output for rule processing
    pub async fn start_pty_monitoring(
        &self,
        agent: Arc<Agent>,
        mut receiver: broadcast::Receiver<String>,
    ) -> Result<()> {
        loop {
            self.receive_and_process_pty_output(&agent, &mut receiver)
                .await?;

            // Small delay to prevent busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    async fn receive_and_process_pty_output(
        &self,
        agent: &Agent,
        receiver: &mut broadcast::Receiver<String>,
    ) -> Result<()> {
        let mut received_any = false;

        while let Ok(pty_output) = receiver.try_recv() {
            received_any = true;
            let current_status = agent.get_status().await;

            tracing::debug!(
                "📝 Agent {} ({:?}) received PTY output: {} bytes: '{}'",
                agent.get_id(),
                current_status,
                pty_output.len(),
                pty_output.chars().take(50).collect::<String>()
            );

            // Process rules only for Active agents
            if current_status == AgentStatus::Active {
                tracing::debug!(
                    "🔍 Processing rules for agent {} ({:?})",
                    agent.get_id(),
                    current_status
                );
                if let Err(e) = self.process_rules_for_output(&pty_output, agent).await {
                    tracing::debug!("❌ Error processing PTY output: {}", e);
                }
            } else {
                tracing::trace!(
                    "⏸️  Skipping rule processing for agent {} (status: {:?})",
                    agent.get_id(),
                    current_status
                );
            }
        }

        if received_any {
            tracing::debug!("✅ Agent {} processed data chunks", agent.get_id());
        }

        Ok(())
    }

    /// Strip ANSI escape sequences from text
    fn strip_ansi_escapes(&self, text: &str) -> String {
        let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap();
        ansi_regex.replace_all(text, "").to_string()
    }
}
