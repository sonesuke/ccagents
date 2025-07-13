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

        tracing::info!("Executing on_start entries...");

        for (i, entry) in self.entries.iter().enumerate() {
            let agent = self.agents.get_agent_by_index(i % self.agents.size());
            tracing::info!(
                "Executing startup entry '{}' on agent {}",
                entry.name,
                agent.get_id()
            );
            entry.execute(&agent).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::config::helper::ActionType;
    use crate::config::triggers_config::{Trigger, TriggerType};

    #[tokio::test]
    async fn test_startup_new() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let entries = vec![];

        let startup = Startup::new(entries, Arc::new(agents));
        assert!(startup.entries.is_empty());
    }

    #[tokio::test]
    async fn test_startup_execute_all_entries_empty() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let entries = vec![];

        let startup = Startup::new(entries, Arc::new(agents));
        let result = startup.execute_all_entries().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_startup_execute_all_entries_with_entries() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();

        let entries = vec![
            Trigger {
                name: "startup1".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["echo".to_string(), "test1".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup2".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["echo".to_string(), "test2".to_string()]),
                source: None,
                dedupe: false,
            },
        ];

        let startup = Startup::new(entries, Arc::new(agents));
        let result = startup.execute_all_entries().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_startup_execute_with_single_agent() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        config.agents.pool = 1;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();

        // Multiple entries but only one agent - should use round-robin
        let entries = vec![
            Trigger {
                name: "startup1".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test1".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup2".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test2".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup3".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test3".to_string()]),
                source: None,
                dedupe: false,
            },
        ];

        let startup = Startup::new(entries, Arc::new(agents));
        let result = startup.execute_all_entries().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_startup_execute_with_multiple_agents() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        config.agents.pool = 3;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();

        let entries = vec![
            Trigger {
                name: "startup1".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test1".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup2".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test2".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup3".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test3".to_string()]),
                source: None,
                dedupe: false,
            },
        ];

        let startup = Startup::new(entries, Arc::new(agents));
        let result = startup.execute_all_entries().await;
        assert!(result.is_ok());
    }
}
