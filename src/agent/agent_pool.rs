use crate::agent::terminal_agent::Agent;
use crate::config::app_config::MonitorConfig;
use anyhow::Result;
use std::sync::Arc;

/// Agent pool for managing multiple agents in parallel
pub struct AgentPool {
    agents: Vec<Arc<Agent>>,
}

impl AgentPool {
    /// Create a new agent pool from monitor configuration
    pub async fn new(monitor_config: &MonitorConfig) -> Result<Self> {
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

        Ok(Self { agents })
    }

    /// Get the number of agents in the pool
    pub fn size(&self) -> usize {
        self.agents.len()
    }

    /// Get agent by index
    pub fn get_agent_by_index(&self, index: usize) -> Arc<Agent> {
        Arc::clone(&self.agents[index % self.agents.len()])
    }
}
