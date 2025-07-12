use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent;
use crate::monitor::{PeriodicTaskManager, StartupTaskManager};
use crate::queue::SharedQueueManager;
use crate::ruler::entry::CompiledEntry;

/// Trigger system responsible for managing startup and periodic entries
pub struct TriggerSystem {
    startup_entries: Vec<CompiledEntry>,
    periodic_entries: Vec<CompiledEntry>,
    agent_pool: Arc<agent::AgentPool>,
    queue_manager: SharedQueueManager,
}

impl TriggerSystem {
    pub fn new(
        startup_entries: Vec<CompiledEntry>,
        periodic_entries: Vec<CompiledEntry>,
        agent_pool: Arc<agent::AgentPool>,
        queue_manager: SharedQueueManager,
    ) -> Self {
        Self {
            startup_entries,
            periodic_entries,
            agent_pool,
            queue_manager,
        }
    }

    /// Start all trigger systems: execute startup entries then start periodic tasks
    pub async fn start_all_triggers(&self) -> Result<Vec<JoinHandle<()>>> {
        // 1. Execute startup entries
        self.execute_startup_entries().await?;

        // 2. Start periodic tasks
        let periodic_handles = self.start_periodic_tasks();

        Ok(periodic_handles)
    }

    async fn execute_startup_entries(&self) -> Result<()> {
        let startup_manager = StartupTaskManager::new(
            self.startup_entries.clone(),
            Arc::clone(&self.agent_pool),
            self.queue_manager.clone(),
        );
        startup_manager.execute_all_entries().await
    }

    fn start_periodic_tasks(&self) -> Vec<JoinHandle<()>> {
        let periodic_manager = PeriodicTaskManager::new(
            self.periodic_entries.clone(),
            Arc::clone(&self.agent_pool),
            self.queue_manager.clone(),
        );
        periodic_manager.start_all_tasks()
    }
}
