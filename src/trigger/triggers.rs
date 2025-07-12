use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent;
use crate::config::TriggerConfig;
use crate::trigger::{PeriodicTaskManager, StartupTaskManager};

/// Triggers responsible for managing startup and periodic entries
pub struct Triggers {
    trigger_config: TriggerConfig,
    agent_pool: Arc<agent::AgentPool>,
}

impl Triggers {
    pub fn new(trigger_config: TriggerConfig, agent_pool: Arc<agent::AgentPool>) -> Self {
        Self {
            trigger_config,
            agent_pool,
        }
    }

    /// Start all triggers: execute startup entries then start periodic tasks
    pub async fn start_all(&self) -> Result<Vec<JoinHandle<()>>> {
        // 1. Execute startup entries
        self.execute_startup_entries().await?;

        // 2. Start periodic tasks
        let periodic_handles = self.start_periodic_tasks().await;

        Ok(periodic_handles)
    }

    async fn execute_startup_entries(&self) -> Result<()> {
        let startup_entries = self.trigger_config.get_on_start_entries().await;
        let startup_manager =
            StartupTaskManager::new(startup_entries, Arc::clone(&self.agent_pool));
        startup_manager.execute_all_entries().await
    }

    async fn start_periodic_tasks(&self) -> Vec<JoinHandle<()>> {
        let periodic_entries = self.trigger_config.get_periodic_entries().await;
        let periodic_manager =
            PeriodicTaskManager::new(periodic_entries, Arc::clone(&self.agent_pool));
        periodic_manager.start_all_tasks()
    }
}
