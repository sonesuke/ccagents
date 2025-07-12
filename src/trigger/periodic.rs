use anyhow::Result;
use std::process::Command;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::interval;

use crate::agent::AgentPool;
use crate::agent::execution::execute_periodic_entry;
use crate::config::entry::{CompiledEntry, TriggerType};

/// Periodic task manager responsible for handling periodic entries
pub struct PeriodicTaskManager {
    pub entries: Vec<CompiledEntry>,
    pub agent_pool: Arc<AgentPool>,
}

impl PeriodicTaskManager {
    pub fn new(entries: Vec<CompiledEntry>, agent_pool: Arc<AgentPool>) -> Self {
        Self {
            entries,
            agent_pool,
        }
    }

    /// Start all periodic tasks and return their handles
    pub fn start_all_tasks(&self) -> Vec<JoinHandle<()>> {
        let mut handles = Vec::new();

        for entry in &self.entries {
            if let TriggerType::Periodic { interval: period } = entry.trigger {
                let handle = self.start_single_task(entry.clone(), period);
                handles.push(handle);
            }
        }

        handles
    }

    fn start_single_task(
        &self,
        entry: CompiledEntry,
        period: tokio::time::Duration,
    ) -> JoinHandle<()> {
        let entry_clone = entry.clone();
        let agent_pool_clone = Arc::clone(&self.agent_pool);

        tokio::spawn(async move {
            // Execute immediately on startup
            println!(
                "⏰ Executing periodic entry immediately on startup: {}",
                entry_clone.name
            );
            let agent = agent_pool_clone.get_agent_by_index(0);
            if let Err(e) = execute_periodic_entry(&entry_clone, Some(&agent)).await {
                eprintln!(
                    "❌ Error executing startup periodic entry '{}': {}",
                    entry_clone.name, e
                );
            }

            // Continue with periodic execution
            let mut timer = interval(period);
            loop {
                timer.tick().await;
                println!("⏰ Executing periodic entry: {}", entry_clone.name);

                // Check if there's data to process
                match has_data_to_process(&entry_clone).await {
                    Ok(true) => {
                        // Execute on available agent
                        let agent = agent_pool_clone.get_agent_by_index(0);
                        if let Err(e) = execute_periodic_entry(&entry_clone, Some(&agent)).await {
                            eprintln!(
                                "❌ Error executing periodic entry '{}': {}",
                                entry_clone.name, e
                            );
                        }
                    }
                    Ok(false) => {
                        // No data to process
                        println!(
                            "ℹ️ No data to process for periodic entry: {}",
                            entry_clone.name
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "❌ Error checking data for periodic entry '{}': {}",
                            entry_clone.name, e
                        );
                    }
                }
            }
        })
    }
}

/// Check if a periodic entry will produce data to process
async fn has_data_to_process(entry: &CompiledEntry) -> Result<bool> {
    // If there's no source command, we consider it as having data to process
    if entry.source.is_none() {
        return Ok(true);
    }

    if let Some(source) = &entry.source {
        // Execute the source command
        let output = Command::new("sh").arg("-c").arg(source).output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.to_string())
            .collect();

        // Return true if we have any lines to process
        Ok(!lines.is_empty())
    } else {
        Ok(true)
    }
}
