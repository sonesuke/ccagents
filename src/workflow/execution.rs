use crate::agent::Agent;
use crate::ruler::rule_types::ActionType;
use anyhow::Result;
use tokio::time::Duration;

pub struct ActionExecutor {
    test_mode: bool,
}

impl ActionExecutor {
    pub fn new(test_mode: bool) -> Self {
        Self { test_mode }
    }

    /// Execute an action based on the ActionType system
    pub async fn execute_action(&self, agent: &Agent, action: ActionType) -> Result<()> {
        match action {
            ActionType::SendKeys(keys) => {
                println!("→ Sending keys to agent {}: {:?}", agent.id(), keys);
                self.send_keys_to_agent(agent, keys).await?;
            }
            ActionType::Workflow(workflow_name, args) => {
                println!(
                    "→ Executing workflow '{}' for agent {} with args: {:?}",
                    workflow_name,
                    agent.id(),
                    args
                );
                self.execute_workflow(agent, &workflow_name, args).await?;
            }
        }
        Ok(())
    }

    /// Send keys directly to the terminal
    async fn send_keys_to_agent(&self, agent: &Agent, keys: Vec<String>) -> Result<()> {
        if self.test_mode {
            println!(
                "ℹ️ Test mode: would send keys {:?} to agent {}",
                keys,
                agent.id()
            );
            return Ok(());
        }

        for key in keys {
            println!("  → Sending key: '{}'", key);
            agent
                .send_keys(&key)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send key '{}': {}", key, e))?;
            // Small delay between keys to avoid overwhelming the terminal
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }

    /// Execute a workflow by name
    async fn execute_workflow(
        &self,
        _agent: &Agent,
        workflow_name: &str,
        _args: Vec<String>,
    ) -> Result<()> {
        // All workflows should be handled by external configuration
        // No hardcoded workflows in the executor
        Err(anyhow::anyhow!(
            "Workflow '{}' not found. Workflows should be defined in external configuration files.",
            workflow_name
        ))
    }
}
