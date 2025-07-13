use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::interval;

use super::execute_shell_command;
use crate::agent::Agents;
use crate::config::triggers_config::{Trigger, TriggerType};

/// Periodic task manager responsible for handling periodic entries
pub struct Periodic {
    pub entries: Vec<Trigger>,
    pub agents: Arc<Agents>,
}

impl Periodic {
    pub fn new(entries: Vec<Trigger>, agents: Arc<Agents>) -> Self {
        Self { entries, agents }
    }
    /// Start all periodic tasks and return their handles
    pub fn start_all_tasks(&self) -> Vec<JoinHandle<()>> {
        self.entries
            .iter()
            .filter_map(|entry| {
                if let TriggerType::Periodic { interval: period } = entry.trigger {
                    let entry = entry.clone();
                    let agents = Arc::clone(&self.agents);

                    Some(tokio::spawn(async move {
                        tracing::debug!("Starting periodic entry: {}", entry.name);

                        // Execute immediately on startup
                        let agent = agents.get_next_agent();
                        if let Err(e) = entry.execute(&agent).await {
                            tracing::error!(
                                "Error executing periodic entry '{}': {}",
                                entry.name,
                                e
                            );
                        }

                        // Continue with periodic execution
                        let mut timer = interval(period);
                        loop {
                            timer.tick().await;

                            match has_data_to_process(&entry).await {
                                Ok(true) => {
                                    let agent = agents.get_next_agent();
                                    if let Err(e) = entry.execute(&agent).await {
                                        tracing::error!(
                                            "Error executing periodic entry '{}': {}",
                                            entry.name,
                                            e
                                        );
                                    }
                                }
                                Ok(false) => {
                                    tracing::debug!("No data to process for entry: {}", entry.name)
                                }
                                Err(e) => tracing::error!(
                                    "Error checking data for entry '{}': {}",
                                    entry.name,
                                    e
                                ),
                            }
                        }
                    }))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Check if a periodic entry will produce data to process
async fn has_data_to_process(entry: &Trigger) -> Result<bool> {
    // If there's no source command, we consider it as having data to process
    let Some(source) = &entry.source else {
        return Ok(true);
    };

    // Execute the source command
    let result = execute_shell_command(source).await?;

    // Return false if command failed or has no output
    Ok(result.success && result.has_output())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::config::helper::ActionType;
    use crate::config::triggers_config::{Trigger, TriggerType};
    use tokio::time::Duration as TokioDuration;

    #[tokio::test]
    async fn test_periodic_new() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let entries = vec![];

        let periodic = Periodic::new(entries, Arc::new(agents));
        assert!(periodic.entries.is_empty());
    }

    #[tokio::test]
    async fn test_periodic_start_all_tasks_empty() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let entries = vec![];

        let periodic = Periodic::new(entries, Arc::new(agents));
        let handles = periodic.start_all_tasks();
        assert!(handles.is_empty());
    }

    #[tokio::test]
    async fn test_periodic_start_all_tasks_with_entries() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();

        let entries = vec![
            Trigger {
                name: "periodic1".to_string(),
                trigger: TriggerType::Periodic {
                    interval: TokioDuration::from_millis(100),
                },
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup1".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "periodic2".to_string(),
                trigger: TriggerType::Periodic {
                    interval: TokioDuration::from_millis(200),
                },
                action: ActionType::SendKeys(vec!["test2".to_string()]),
                source: None,
                dedupe: false,
            },
        ];

        let periodic = Periodic::new(entries, Arc::new(agents));
        let handles = periodic.start_all_tasks();

        // Should only start periodic tasks, not startup tasks
        assert_eq!(handles.len(), 2);

        // Clean up handles
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn test_has_data_to_process_no_source() {
        let trigger = Trigger {
            name: "test".to_string(),
            trigger: TriggerType::Periodic {
                interval: TokioDuration::from_secs(1),
            },
            action: ActionType::SendKeys(vec!["test".to_string()]),
            source: None,
            dedupe: false,
        };

        let result = has_data_to_process(&trigger).await.unwrap();
        assert!(result); // Should return true when no source command
    }

    #[tokio::test]
    async fn test_has_data_to_process_with_successful_source() {
        let trigger = Trigger {
            name: "test".to_string(),
            trigger: TriggerType::Periodic {
                interval: TokioDuration::from_secs(1),
            },
            action: ActionType::SendKeys(vec!["test".to_string()]),
            source: Some("echo hello".to_string()),
            dedupe: false,
        };

        let result = has_data_to_process(&trigger).await.unwrap();
        assert!(result); // Should return true when source produces output
    }

    #[tokio::test]
    async fn test_has_data_to_process_with_failed_source() {
        let trigger = Trigger {
            name: "test".to_string(),
            trigger: TriggerType::Periodic {
                interval: TokioDuration::from_secs(1),
            },
            action: ActionType::SendKeys(vec!["test".to_string()]),
            source: Some("false".to_string()),
            dedupe: false,
        };

        let result = has_data_to_process(&trigger).await.unwrap();
        assert!(!result); // Should return false when command fails
    }

    #[tokio::test]
    async fn test_has_data_to_process_with_no_output_source() {
        let trigger = Trigger {
            name: "test".to_string(),
            trigger: TriggerType::Periodic {
                interval: TokioDuration::from_secs(1),
            },
            action: ActionType::SendKeys(vec!["test".to_string()]),
            source: Some("true".to_string()), // succeeds but produces no output
            dedupe: false,
        };

        let result = has_data_to_process(&trigger).await.unwrap();
        assert!(!result); // Should return false when no output
    }
}
