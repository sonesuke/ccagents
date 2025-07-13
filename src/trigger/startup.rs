use anyhow::Result;
use std::sync::Arc;

use crate::agent::Agents;
use crate::config::triggers_config::Trigger;

/// Startup task manager responsible for handling on_start entries
pub struct Startup {
    pub entries: Vec<Trigger>,
    pub agents: Arc<Agents>,
}

impl Startup {
    pub fn new(entries: Vec<Trigger>, agents: Arc<Agents>) -> Self {
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
            entry.execute(&agent).await?;
        }

        Ok(())
    }
}
