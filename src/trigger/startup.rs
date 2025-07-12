use anyhow::Result;
use std::sync::Arc;

use crate::agent;
use crate::cli::execute_entry_action;
use crate::config::entry::CompiledEntry;

/// Startup task manager responsible for handling on_start entries
pub struct StartupTaskManager {
    pub entries: Vec<CompiledEntry>,
    pub agent_pool: Arc<agent::AgentPool>,
}

impl StartupTaskManager {
    pub fn new(entries: Vec<CompiledEntry>, agent_pool: Arc<agent::AgentPool>) -> Self {
        Self {
            entries,
            agent_pool,
        }
    }

    /// Execute all startup entries
    pub async fn execute_all_entries(&self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        println!("🎬 Executing on_start entries...");

        for (i, entry) in self.entries.iter().enumerate() {
            let agent = self
                .agent_pool
                .get_agent_by_index(i % self.agent_pool.size());
            println!(
                "🎯 Executing startup entry '{}' on agent {}",
                entry.name,
                agent.get_id()
            );
            execute_entry_action(&agent, entry).await?;
        }

        Ok(())
    }
}
