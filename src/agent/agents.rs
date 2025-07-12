use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent::Agent;
use crate::config::RuleConfig;
use crate::config::loader::MonitorConfig;
use crate::rule::{DiffTimeout, When};

/// Agents responsible for managing agent pool and monitoring agents
pub struct Agents {
    rule_config: RuleConfig,
    agents: Vec<Arc<Agent>>,
}

impl Agents {
    /// Create a new agents system from monitor configuration
    pub async fn new(rule_config: RuleConfig, monitor_config: &MonitorConfig) -> Result<Self> {
        let mut agents = Vec::new();
        let pool_size = monitor_config.get_agent_pool_size();
        let base_port = monitor_config.get_web_ui_port();
        let test_mode = crate::config::is_test_mode();

        for i in 0..pool_size {
            let port = base_port + i as u16;
            let agent_id = format!("agent-{}", i);
            let (cols, rows) = monitor_config.get_agent_dimensions(i);
            let agent = Arc::new(
                Agent::new(
                    agent_id,
                    test_mode,
                    port,
                    cols,
                    rows,
                    monitor_config.web_ui.host.clone(),
                    monitor_config.web_ui.enabled,
                )
                .await?,
            );

            // Start web server if enabled
            if monitor_config.web_ui.enabled {
                agent
                    .clone()
                    .start_web_server(port, monitor_config.web_ui.host.clone())
                    .await?;
            }

            agents.push(agent);
        }

        Ok(Self {
            rule_config,
            agents,
        })
    }

    /// Get the number of agents in the pool
    pub fn size(&self) -> usize {
        self.agents.len()
    }

    /// Get agent by index
    pub fn get_agent_by_index(&self, index: usize) -> Arc<Agent> {
        Arc::clone(&self.agents[index % self.agents.len()])
    }

    /// Start all monitoring systems: agent monitors and timeout monitor
    pub async fn start_all(&self) -> Result<Vec<JoinHandle<()>>> {
        let mut monitoring_handles = Vec::new();

        // Create agent monitors for each agent
        for i in 0..self.size() {
            let agent = self.get_agent_by_index(i);

            // Get PTY receiver for this agent
            match agent.get_pty_string_receiver().await {
                Ok(receiver) => {
                    tracing::info!(
                        "✅ Agent {} persistent string receiver created",
                        agent.get_id()
                    );

                    // Start agent status monitoring (independent of PTY output)
                    let status_agent = Arc::clone(&agent);
                    let status_handle = tokio::spawn(async move {
                        if let Err(e) = status_agent.start_status_monitoring().await {
                            tracing::error!("❌ Agent status monitor failed: {}", e);
                        }
                    });
                    monitoring_handles.push(status_handle);

                    // Start PTY output monitoring for when condition processing
                    let when_processor = When::new(self.rule_config.clone());
                    let pty_agent = Arc::clone(&agent);
                    let pty_handle = tokio::spawn(async move {
                        if let Err(e) = when_processor
                            .start_pty_monitoring(pty_agent, receiver)
                            .await
                        {
                            tracing::error!("❌ PTY monitor failed: {}", e);
                        }
                    });
                    monitoring_handles.push(pty_handle);
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

        // Create diff_timeout monitor - pass agents as a reference
        let agents_arc = Arc::new(self.agents.clone());
        let diff_timeout_monitor = DiffTimeout {
            rule_config: self.rule_config.clone(),
            agents: agents_arc,
        };

        let timeout_handle = tokio::spawn(async move {
            if let Err(e) = diff_timeout_monitor.start_monitoring().await {
                tracing::error!("❌ Diff timeout monitor failed: {}", e);
            }
        });
        monitoring_handles.push(timeout_handle);

        Ok(monitoring_handles)
    }
}
