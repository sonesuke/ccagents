use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::Duration;

use crate::agent::pty_processor::process_pty_output;
use crate::agent::terminal_agent::{Agent, AgentStatus};
use crate::config::RuleConfig;

use super::Monitor;

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
            self.process_pty_output(current_status).await?;

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn process_pty_output(&mut self, status: AgentStatus) -> Result<()> {
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
                if let Err(e) =
                    process_pty_output(&pty_output, &self.agent, &self.rule_config).await
                {
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
}
