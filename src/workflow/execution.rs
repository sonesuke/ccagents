use crate::agent::Agent;
use crate::queue::SharedQueueManager;
use crate::ruler::types::ActionType;
use anyhow::Result;
use tokio::time::Duration;

#[allow(dead_code)]
pub struct ActionExecutor {
    test_mode: bool,
    queue_manager: Option<SharedQueueManager>,
}

impl ActionExecutor {
    #[allow(dead_code)]
    pub fn new(test_mode: bool) -> Self {
        Self {
            test_mode,
            queue_manager: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_queue_manager(test_mode: bool, queue_manager: SharedQueueManager) -> Self {
        Self {
            test_mode,
            queue_manager: Some(queue_manager),
        }
    }

    /// Execute an action based on the ActionType system
    #[allow(dead_code)]
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
            ActionType::Enqueue { queue, command } => {
                println!(
                    "→ Executing command '{}' and enqueuing to '{}' for agent {}",
                    command,
                    queue,
                    agent.id()
                );
                self.execute_and_enqueue(&queue, &command).await?;
            }
            ActionType::EnqueueDedupe { queue, command } => {
                println!(
                    "→ Executing command '{}' and enqueuing to dedupe '{}' for agent {}",
                    command,
                    queue,
                    agent.id()
                );
                self.execute_and_enqueue_dedupe(&queue, &command).await?;
            }
        }
        Ok(())
    }

    /// Send keys directly to the terminal
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    /// Execute a command and enqueue its output to a queue
    #[allow(dead_code)]
    async fn execute_and_enqueue(&self, queue_name: &str, command: &str) -> Result<()> {
        if self.test_mode {
            println!(
                "ℹ️ Test mode: would execute command '{}' and enqueue to '{}'",
                command, queue_name
            );
            return Ok(());
        }

        let queue_manager = self
            .queue_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Queue manager not initialized"))?;

        // Use QueueExecutor for command execution
        let executor = crate::queue::QueueExecutor::new(queue_manager.clone());
        let count = executor.execute_and_enqueue(queue_name, command).await?;

        println!("✅ Enqueued {} items to queue '{}'", count, queue_name);
        Ok(())
    }

    #[allow(dead_code)]
    async fn execute_and_enqueue_dedupe(&self, queue_name: &str, command: &str) -> Result<()> {
        if self.test_mode {
            println!(
                "ℹ️ Test mode: would execute command '{}' and enqueue to dedupe '{}'",
                command, queue_name
            );
            return Ok(());
        }

        let queue_manager = self
            .queue_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Queue manager not initialized"))?;

        // Use QueueExecutor for command execution with deduplication
        let executor = crate::queue::QueueExecutor::new(queue_manager.clone());
        let count = executor
            .execute_and_enqueue_dedupe(queue_name, command)
            .await?;

        println!(
            "✅ Enqueued {} new items to dedupe queue '{}'",
            count, queue_name
        );
        Ok(())
    }
}
