use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent;
use crate::monitor::{AgentMonitor, TimeoutMonitor};
use crate::queue::SharedQueueManager;
use crate::ruler::Ruler;

/// Agent system responsible for monitoring agents and processing rules
pub struct AgentSystem {
    ruler: Arc<Ruler>,
    agent_pool: Arc<agent::AgentPool>,
    queue_manager: SharedQueueManager,
}

impl AgentSystem {
    pub fn new(
        ruler: Arc<Ruler>,
        agent_pool: Arc<agent::AgentPool>,
        queue_manager: SharedQueueManager,
    ) -> Self {
        Self {
            ruler,
            agent_pool,
            queue_manager,
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
                        ruler: Arc::clone(&self.ruler),
                        queue_manager: self.queue_manager.clone(),
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
            ruler: Arc::clone(&self.ruler),
            agent_pool: Arc::clone(&self.agent_pool),
            queue_manager: self.queue_manager.clone(),
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
