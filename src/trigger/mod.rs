use anyhow::Result;
use std::process::Command;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

use crate::agent::{Agent, Agents};
use crate::config;
use crate::config::triggers_config::{Trigger, TriggerType};

pub mod periodic;
pub mod startup;

pub use periodic::Periodic;
pub use startup::Startup;

/// Result of executing a shell command
#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    /// Get non-empty lines from stdout
    pub fn stdout_lines(&self) -> Vec<String> {
        self.stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if command produced any output lines
    pub fn has_output(&self) -> bool {
        !self.stdout_lines().is_empty()
    }
}

/// Execute a shell command and return structured result
pub async fn execute_shell_command(command: &str) -> Result<CommandResult> {
    tracing::debug!("Executing shell command: {}", command);

    let output = Command::new("sh").arg("-c").arg(command).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    if !success {
        tracing::warn!(
            "Command '{}' failed with stderr: {}",
            command,
            stderr.trim()
        );
    }

    Ok(CommandResult {
        success,
        stdout,
        stderr,
    })
}

impl config::triggers_config::Trigger {
    /// Execute this trigger using the provided agent
    pub async fn execute(&self, agent: &Agent) -> Result<()> {
        tracing::info!("📦 Executing entry '{}': {:?}", self.name, self.action);

        if let Some(source) = &self.source {
            self.execute_source_command(source, agent).await
        } else {
            self.execute_action(agent, &format!("Entry '{}'", self.name))
                .await
        }
    }

    /// Execute a source command and process its output
    async fn execute_source_command(&self, source: &str, agent: &Agent) -> Result<()> {
        let result = execute_shell_command(source).await?;

        if !result.success {
            anyhow::bail!(
                "Source command failed: {} (stderr: {})",
                source,
                result.stderr.trim()
            );
        }

        let lines = result.stdout_lines();
        if lines.is_empty() {
            tracing::info!("Source command '{}' produced no output", source);
            return Ok(());
        }

        tracing::info!("Source command '{}' produced {} lines", source, lines.len());

        // Process each line from the source command
        for (i, line) in lines.iter().enumerate() {
            let resolved_action = resolve_placeholders(&self.action, line);
            let context = format!("Source line {}/{}", i + 1, lines.len());

            tracing::debug!(
                "{}: {}",
                context,
                line.chars().take(100).collect::<String>()
            );

            if let Err(e) = execute_action_with_agent(&resolved_action, agent, &context).await {
                tracing::error!("Failed to process {}: {}", context, e);
            }

            // Small delay between lines to prevent overwhelming the system
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Execute this trigger's action using the provided agent
    async fn execute_action(&self, agent: &Agent, context: &str) -> Result<()> {
        execute_action_with_agent(&self.action, agent, context).await
    }
}

/// Execute an action with consistent 100ms delay between keys
async fn execute_action_with_agent(
    action: &config::helper::ActionType,
    agent: &Agent,
    context: &str,
) -> Result<()> {
    let config::helper::ActionType::SendKeys(keys) = action;
    if keys.is_empty() {
        tracing::debug!("{}: No keys to send", context);
        return Ok(());
    }

    tracing::info!("{}: Sending {} keys", context, keys.len());
    tracing::debug!("{}: Keys: {:?}", context, keys);

    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        agent.send_keys(key).await?;
    }

    Ok(())
}

/// Resolve ${1} placeholders in action with source line content
fn resolve_placeholders(
    action: &config::helper::ActionType,
    line: &str,
) -> config::helper::ActionType {
    match action {
        config::helper::ActionType::SendKeys(keys) => {
            let resolved_keys = keys.iter().map(|key| key.replace("${1}", line)).collect();
            config::helper::ActionType::SendKeys(resolved_keys)
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::config::helper::ActionType;
    use crate::config::triggers_config::{Trigger, TriggerType};
    use tokio::time::Duration as TokioDuration;

    #[tokio::test]
    async fn test_execute_shell_command_success() {
        let result = execute_shell_command("echo hello").await.unwrap();
        assert!(result.success);
        assert_eq!(result.stdout_lines(), vec!["hello"]);
        assert!(result.has_output());
    }

    #[tokio::test]
    async fn test_execute_shell_command_failure() {
        let result = execute_shell_command("false").await.unwrap();
        assert!(!result.success);
        assert!(!result.has_output());
    }

    #[tokio::test]
    async fn test_execute_shell_command_with_stderr() {
        let result = execute_shell_command("echo error >&2; false")
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.stderr.contains("error"));
    }

    #[test]
    fn test_command_result_stdout_lines() {
        let result = CommandResult {
            success: true,
            stdout: "line1\n\nline2\n".to_string(),
            stderr: String::new(),
        };
        assert_eq!(result.stdout_lines(), vec!["line1", "line2"]);
    }

    #[test]
    fn test_command_result_has_output() {
        let result_with_output = CommandResult {
            success: true,
            stdout: "content".to_string(),
            stderr: String::new(),
        };
        assert!(result_with_output.has_output());

        let result_no_output = CommandResult {
            success: true,
            stdout: "\n\n".to_string(),
            stderr: String::new(),
        };
        assert!(!result_no_output.has_output());
    }

    #[test]
    fn test_get_startup_triggers() {
        let triggers = vec![
            Trigger {
                name: "startup1".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "periodic1".to_string(),
                trigger: TriggerType::Periodic {
                    interval: TokioDuration::from_secs(1),
                },
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "startup2".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
        ];

        let startup_triggers = get_startup_triggers(&triggers);
        assert_eq!(startup_triggers.len(), 2);
        assert_eq!(startup_triggers[0].name, "startup1");
        assert_eq!(startup_triggers[1].name, "startup2");
    }

    #[test]
    fn test_get_periodic_triggers() {
        let triggers = vec![
            Trigger {
                name: "startup1".to_string(),
                trigger: TriggerType::OnStart,
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "periodic1".to_string(),
                trigger: TriggerType::Periodic {
                    interval: TokioDuration::from_secs(1),
                },
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
            Trigger {
                name: "periodic2".to_string(),
                trigger: TriggerType::Periodic {
                    interval: TokioDuration::from_secs(2),
                },
                action: ActionType::SendKeys(vec!["test".to_string()]),
                source: None,
                dedupe: false,
            },
        ];

        let periodic_triggers = get_periodic_triggers(&triggers);
        assert_eq!(periodic_triggers.len(), 2);
        assert_eq!(periodic_triggers[0].name, "periodic1");
        assert_eq!(periodic_triggers[1].name, "periodic2");
    }

    #[tokio::test]
    async fn test_triggers_new() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let triggers = vec![];

        let trigger_manager = Triggers::new(triggers, Arc::new(agents));
        assert_eq!(trigger_manager.triggers.len(), 0);
    }

    #[tokio::test]
    async fn test_triggers_start_all_empty() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let triggers = vec![];

        let trigger_manager = Triggers::new(triggers, Arc::new(agents));
        let handles = trigger_manager.start_all().await.unwrap();
        assert!(handles.is_empty());
    }

    #[test]
    fn test_resolve_placeholders() {
        let action = ActionType::SendKeys(vec![
            "echo".to_string(),
            "${1}".to_string(),
            "done".to_string(),
        ]);

        let resolved = resolve_placeholders(&action, "test_value");

        let ActionType::SendKeys(keys) = resolved;
        assert_eq!(keys, vec!["echo", "test_value", "done"]);
    }

    #[tokio::test]
    async fn test_trigger_execute_simple_action() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let agent = agents.get_agent_by_index(0);

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            trigger: TriggerType::OnStart,
            action: ActionType::SendKeys(vec!["echo".to_string(), "test".to_string()]),
            source: None,
            dedupe: false,
        };

        let result = trigger.execute(&agent).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_execute_with_source_success() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let agent = agents.get_agent_by_index(0);

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            trigger: TriggerType::OnStart,
            action: ActionType::SendKeys(vec!["echo".to_string(), "${1}".to_string()]),
            source: Some("echo hello".to_string()),
            dedupe: false,
        };

        let result = trigger.execute(&agent).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_execute_with_source_failure() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let agent = agents.get_agent_by_index(0);

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            trigger: TriggerType::OnStart,
            action: ActionType::SendKeys(vec!["echo".to_string(), "${1}".to_string()]),
            source: Some("false".to_string()),
            dedupe: false,
        };

        let result = trigger.execute(&agent).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_trigger_execute_with_source_no_output() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let agent = agents.get_agent_by_index(0);

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            trigger: TriggerType::OnStart,
            action: ActionType::SendKeys(vec!["echo".to_string(), "${1}".to_string()]),
            source: Some("true".to_string()), // succeeds but produces no output
            dedupe: false,
        };

        let result = trigger.execute(&agent).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_action_with_agent_empty_keys() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let agent = agents.get_agent_by_index(0);

        let action = ActionType::SendKeys(vec![]);
        let result = execute_action_with_agent(&action, &agent, "test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_action_with_agent_with_keys() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        let agents = Agents::new_with_mock(vec![], &config).await.unwrap();
        let agent = agents.get_agent_by_index(0);

        let action = ActionType::SendKeys(vec!["echo".to_string(), "test".to_string()]);
        let result = execute_action_with_agent(&action, &agent, "test").await;
        assert!(result.is_ok());
    }
}
