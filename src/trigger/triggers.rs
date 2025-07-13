use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent::Agents;
use crate::config::triggers_config::{Trigger, TriggerType};
use crate::trigger::{Periodic, Startup};

/// Triggers responsible for managing startup and periodic entries
pub struct Triggers {
    triggers: Vec<Trigger>,
    agents: Arc<Agents>,
}

impl Triggers {
    pub fn new(triggers: Vec<Trigger>, agents: Arc<Agents>) -> Self {
        Self { triggers, agents }
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
        let startup_entries = get_startup_triggers(&self.triggers);
        let startup_manager = Startup::new(startup_entries, Arc::clone(&self.agents));
        startup_manager.execute_all_entries().await
    }

    async fn start_periodic_tasks(&self) -> Vec<JoinHandle<()>> {
        let periodic_entries = get_periodic_triggers(&self.triggers);
        let periodic_manager = Periodic::new(periodic_entries, Arc::clone(&self.agents));
        periodic_manager.start_all_tasks()
    }
}

/// Get startup triggers from a list of triggers
pub fn get_startup_triggers(triggers: &[Trigger]) -> Vec<Trigger> {
    triggers
        .iter()
        .filter(|trigger| trigger.trigger == TriggerType::OnStart)
        .cloned()
        .collect()
}

/// Get periodic triggers from a list of triggers
pub fn get_periodic_triggers(triggers: &[Trigger]) -> Vec<Trigger> {
    triggers
        .iter()
        .filter(|trigger| matches!(trigger.trigger, TriggerType::Periodic { .. }))
        .cloned()
        .collect()
}
