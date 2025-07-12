use anyhow::Result;
use std::sync::Arc;
use tokio::time::Duration;

use crate::agent;
use crate::agent::Agent;
use crate::agent::execution::execute_rule_action;
use crate::config::RuleConfig;

use super::Monitor;

/// Timeout monitor responsible for checking timeout rules across all agents
pub struct TimeoutMonitor {
    pub rule_config: RuleConfig,
    pub agents: Arc<Vec<Arc<Agent>>>,
}

impl Monitor for TimeoutMonitor {
    async fn start_monitoring(self) -> Result<()> {
        self.start_monitoring().await
    }
}

impl TimeoutMonitor {
    pub async fn start_monitoring(self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            // Check timeout rules for active agents only
            self.check_timeout_rules().await?;
        }
    }

    async fn check_timeout_rules(&self) -> Result<()> {
        // Only process timeout rules when there are active agents
        for agent in self.agents.iter() {
            let current_status = agent.get_status().await;

            if current_status == agent::AgentStatus::Active {
                let timeout_actions = self.rule_config.check_timeout_rules().await;
                for action in timeout_actions {
                    tracing::info!("⏰ Executing timeout rule action: {:?}", action);
                    if let Err(e) = execute_rule_action(&action, agent).await {
                        tracing::error!("❌ Error executing timeout rule action: {}", e);
                    }
                }
                break; // Only need to check once per cycle if any agent is active
            }
        }

        Ok(())
    }
}
