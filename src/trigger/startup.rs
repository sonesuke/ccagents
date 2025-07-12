use anyhow::Result;
use std::sync::Arc;

use crate::agent::Agents;
use crate::agent::execution::execute_entry_action;
use crate::config::trigger::CompiledEntry;

/// Startup task manager responsible for handling on_start entries
pub struct StartupTaskManager {
    pub entries: Vec<CompiledEntry>,
    pub agents: Arc<Agents>,
}

impl StartupTaskManager {
    pub fn new(entries: Vec<CompiledEntry>, agents: Arc<Agents>) -> Self {
        Self { entries, agents }
    }

    /// Execute all startup entries
    pub async fn execute_all_entries(&self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        println!("🎬 Executing on_start entries...");

        for (i, entry) in self.entries.iter().enumerate() {
            let agent = self.agents.get_agent_by_index(i % self.agents.size());
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
