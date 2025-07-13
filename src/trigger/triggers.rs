use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent::Agents;
use crate::config::trigger::TriggerManager;
use crate::trigger::{Periodic, Startup};

/// Triggers responsible for managing startup and periodic entries
pub struct Triggers {
    trigger_manager: Arc<TriggerManager>,
    agents: Arc<Agents>,
}

impl Triggers {
    pub fn new(trigger_manager: &TriggerManager, agents: Arc<Agents>) -> Self {
        Self {
            trigger_manager: Arc::new(trigger_manager.clone()),
            agents,
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
        let startup_entries = self.trigger_manager.get_on_start_triggers().await;
        let startup_manager = Startup::new(startup_entries, Arc::clone(&self.agents));
        startup_manager.execute_all_entries().await
    }

    async fn start_periodic_tasks(&self) -> Vec<JoinHandle<()>> {
        let periodic_entries = self.trigger_manager.get_periodic_triggers().await;
        let periodic_manager = Periodic::new(periodic_entries, Arc::clone(&self.agents));
        periodic_manager.start_all_tasks()
    }
}
