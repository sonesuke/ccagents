use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent::AgentPool;
use crate::agent::{AgentMonitor, TimeoutMonitor};
use crate::config::RuleConfig;

/// Agents responsible for monitoring agents and processing rules
pub struct Agents {
    rule_config: RuleConfig,
    agent_pool: Arc<AgentPool>,
}

impl Agents {
    pub fn new(rule_config: RuleConfig, agent_pool: Arc<AgentPool>) -> Self {
        Self {
            rule_config,
            agent_pool,
        }
    }

    /// Start all monitoring systems: agent monitors and timeout monitor
    pub async fn start_monitoring(&self) -> Result<Vec<JoinHandle<()>>> {
        let mut monitoring_handles = Vec::new();

        // Create agent monitors for each agent
        for i in 0..self.agent_pool.size() {
            let agent = self.agent_pool.get_agent_by_index(i);

            // Get PTY receiver for this agent
            match agent.get_pty_string_receiver().await {
                Ok(receiver) => {
                    tracing::info!(
                        "✅ Agent {} persistent string receiver created",
                        agent.get_id()
                    );

                    let monitor = AgentMonitor {
                        rule_config: self.rule_config.clone(), // RuleConfigはCloneする必要があります
                        agent,
                        receiver,
                    };

                    let handle = tokio::spawn(async move {
                        if let Err(e) = monitor.start_monitoring().await {
                            tracing::error!("❌ Agent monitor failed: {}", e);
                        }
                    });
                    monitoring_handles.push(handle);
                }
                Err(e) => {
                    tracing::error!(
                        "❌ Agent {} failed to create string receiver: {}",
                        agent.get_id(),
                        e
                    );
                }
            }
        }

        // Create timeout monitor
        let timeout_monitor = TimeoutMonitor {
            rule_config: self.rule_config.clone(),
            agent_pool: Arc::clone(&self.agent_pool),
        };

        let timeout_handle = tokio::spawn(async move {
            if let Err(e) = timeout_monitor.start_monitoring().await {
                tracing::error!("❌ Timeout monitor failed: {}", e);
            }
        });
        monitoring_handles.push(timeout_handle);

        Ok(monitoring_handles)
    }
}
