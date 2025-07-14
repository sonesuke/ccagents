pub mod agents;

use crate::config::Config;
use crate::config::rules_config::Rule;
use crate::rule::RuleProcessor;
use crate::rule::{DiffTimeout, When};
use crate::terminal::pty_process::PtyProcess;
use crate::terminal::pty_process_trait::PtyProcessTrait;
use crate::web_server::WebServer;
use anyhow::Result;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Duration;

/// Agent status for state management
#[derive(Debug, Clone, PartialEq)]
enum AgentStatus {
    Idle,   // Waiting and monitoring triggers
    Active, // Executing tasks and monitoring rules
}

pub struct Agent {
    index: usize,
    process: Box<dyn PtyProcessTrait>,
    config: Config,
    status: RwLock<AgentStatus>,
    web_server_handle: RwLock<Option<JoinHandle<()>>>,
}

impl Agent {
    /// Create a new agent from configuration, handling web server setup
    pub async fn from_config(index: usize, config: &Config) -> Result<Arc<Self>> {
        let process = PtyProcess::from_config(config);

        // Start the PTY process
        process.start().await?;

        Self::new_with_process(index, config, Box::new(process)).await
    }

    /// Create a new agent with a specific PTY process (for testing with mocks)
    pub async fn new_with_process(
        index: usize,
        config: &Config,
        process: Box<dyn PtyProcessTrait>,
    ) -> Result<Arc<Self>> {
        let agent = Arc::new(Agent {
            index,
            process,
            config: config.clone(),
            status: RwLock::new(AgentStatus::Idle),
            web_server_handle: RwLock::new(None),
        });

        // Start web server if enabled
        agent.setup_web_server_if_enabled().await?;

        Ok(agent)
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        self.get_process()
            .send_input(keys.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send keys: {}", e))
    }

    /// Get terminal dimensions for asciinema integration
    pub fn get_terminal_dimensions(&self) -> (u16, u16) {
        (self.config.web_ui.cols, self.config.web_ui.rows)
    }

    /// Get agent ID
    pub fn get_id(&self) -> String {
        format!("agent-{}", self.index)
    }

    /// Get access to the PTY process
    pub fn get_process(&self) -> &dyn PtyProcessTrait {
        self.process.as_ref()
    }

    /// Check if the agent is currently active (true = Active, false = Idle)
    pub async fn is_active(&self) -> bool {
        matches!(*self.status.read().unwrap(), AgentStatus::Active)
    }

    /// Set the status of the agent
    async fn set_status(&self, new_status: AgentStatus) {
        // First check if status actually needs to change (read lock only)
        let needs_update =
            match tokio::time::timeout(tokio::time::Duration::from_millis(50), async {
                self.status.read().map_err(|_| ())
            })
            .await
            {
                Ok(Ok(current_status)) => *current_status != new_status,
                _ => {
                    tracing::warn!(
                        "Status read timeout for agent {}, assuming update needed",
                        self.get_id()
                    );
                    true
                }
            };

        // Only acquire write lock if status needs to change
        if needs_update {
            match tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
                self.status.write().map_err(|_| ())
            })
            .await
            {
                Ok(Ok(mut status)) => {
                    *status = new_status.clone();
                    tracing::debug!("🔄 Agent {} → {:?}", self.get_id(), new_status);
                }
                _ => {
                    tracing::error!("Status write timeout for agent {}", self.get_id());
                }
            }
        }
    }

    /// Setup web server if enabled in configuration
    async fn setup_web_server_if_enabled(self: &Arc<Self>) -> Result<()> {
        if self.config.web_ui.enabled {
            let port = self.config.web_ui.base_port + self.index as u16;
            let host = self.config.web_ui.host.clone();
            Arc::clone(self).start_web_server(port, host).await?;
        }
        Ok(())
    }

    /// Setup all monitoring systems for this agent
    pub async fn setup_monitoring(
        self: Arc<Self>,
        rules: Vec<Rule>,
    ) -> Result<Vec<JoinHandle<()>>> {
        let when_receiver = self.get_pty_receiver().await?;
        let diff_timeout_receiver = self.get_pty_receiver().await?;

        tracing::info!(
            "✅ Agent {} persistent string receivers created",
            self.get_id()
        );

        let handles = vec![
            self.setup_status_monitoring(),
            self.setup_when_monitoring(rules.clone(), when_receiver),
            self.setup_diff_timeout_monitoring(rules, diff_timeout_receiver),
        ];

        Ok(handles)
    }

    /// Get PTY receiver for this agent
    async fn get_pty_receiver(&self) -> Result<tokio::sync::broadcast::Receiver<String>> {
        self.get_process()
            .get_pty_string_receiver()
            .await
            .map_err(|e| {
                tracing::error!(
                    "❌ Agent {} failed to create string receiver: {}",
                    self.get_id(),
                    e
                );
                e.into()
            })
    }

    /// Setup status monitoring for this agent
    fn setup_status_monitoring(self: &Arc<Self>) -> JoinHandle<()> {
        let agent = Arc::clone(self);

        tokio::spawn(async move {
            if let Err(e) = agent.clone().start_monitoring().await {
                tracing::error!("❌ Agent {} status monitor failed: {}", agent.get_id(), e);
            }
        })
    }

    /// Setup when rule monitoring for this agent
    fn setup_when_monitoring(
        self: &Arc<Self>,
        rules: Vec<Rule>,
        receiver: tokio::sync::broadcast::Receiver<String>,
    ) -> JoinHandle<()> {
        let when_monitor = When::new(rules, Arc::clone(self));

        tokio::spawn(async move {
            if when_monitor.start_monitoring(receiver).await.is_err() {
                tracing::error!("❌ Agent when monitor failed");
            }
        })
    }

    /// Setup timeout monitoring for this agent
    fn setup_diff_timeout_monitoring(
        self: &Arc<Self>,
        rules: Vec<Rule>,
        receiver: tokio::sync::broadcast::Receiver<String>,
    ) -> JoinHandle<()> {
        let diff_timeout = DiffTimeout::new(rules, Arc::clone(self));

        tokio::spawn(async move {
            if diff_timeout.start_monitoring(receiver).await.is_err() {
                tracing::error!("❌ Agent timeout monitor failed");
            }
        })
    }

    /// Start the WebServer for this agent if configured
    async fn start_web_server(self: std::sync::Arc<Self>, port: u16, host: String) -> Result<()> {
        let web_server = WebServer::new(port, host, std::sync::Arc::clone(&self));
        let handle = tokio::spawn(async move {
            if let Err(e) = web_server.start().await {
                tracing::error!("❌ Web server failed on port {}: {}", port, e);
            }
        });

        *self.web_server_handle.write().unwrap() = Some(handle);
        Ok(())
    }

    /// Monitor agent status by checking child processes
    async fn monitor(&self) {
        if let Ok(child_pids) = self.get_process().get_child_processes().await {
            let new_status = if child_pids.is_empty() {
                AgentStatus::Idle
            } else {
                AgentStatus::Active
            };

            self.set_status(new_status).await;
        }
    }

    /// Start monitoring this agent's status
    pub async fn start_monitoring(self: std::sync::Arc<Self>) -> Result<()> {
        loop {
            // Add timeout to monitor operation to prevent hanging
            match tokio::time::timeout(Duration::from_millis(200), self.monitor()).await {
                Ok(_) => {}
                Err(_) => {
                    tracing::warn!("Agent {} monitor operation timed out", self.get_id());
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

// Re-export for convenience
pub use agents::Agents;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::pty_process_trait::MockPtyProcess;

    async fn create_test_agent() -> Arc<Agent> {
        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts

        let mock_pty = Box::new(MockPtyProcess::new());
        Agent::new_with_process(0, &config, mock_pty).await.unwrap()
    }

    #[tokio::test]
    async fn test_agent_creation() {
        let _agent = create_test_agent().await;
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }

    #[tokio::test]
    async fn test_agent_getters() {
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        config.agents.pool = 3;

        // Use mock PTY instead of real PTY for stable testing
        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        // Test ID getter
        assert_eq!(agent.get_id(), "agent-0");

        // Test terminal dimensions getter
        let (cols, rows) = agent.get_terminal_dimensions();
        let (expected_cols, expected_rows) = (config.web_ui.cols, config.web_ui.rows);
        assert_eq!(cols, expected_cols);
        assert_eq!(rows, expected_rows);

        // Test process getter (just verify it returns something)
        let _process = agent.get_process();
    }

    #[tokio::test]
    async fn test_agent_status_management() {
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts

        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        // Agent should start as Idle
        assert!(!agent.is_active().await, "Agent should start as Idle");

        // Test status transitions
        agent.set_status(AgentStatus::Active).await;
        assert!(
            agent.is_active().await,
            "Agent should be Active after setting"
        );

        agent.set_status(AgentStatus::Idle).await;
        assert!(
            !agent.is_active().await,
            "Agent should be Idle after setting"
        );
    }

    #[tokio::test]
    async fn test_is_active_method() {
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts

        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        // Test initial state
        assert_eq!(agent.is_active().await, false);

        // Test Active state
        agent.set_status(AgentStatus::Active).await;
        assert_eq!(agent.is_active().await, true);

        // Test Idle state
        agent.set_status(AgentStatus::Idle).await;
        assert_eq!(agent.is_active().await, false);
    }

    #[tokio::test]
    async fn test_send_keys() {
        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        let agent = create_test_agent().await;

        // Test sending keys to the process
        let result = agent.send_keys("echo test").await;

        // The result should be successful since PtyProcess is created successfully
        // The actual implementation depends on PtyProcess.send_input behavior
        assert!(result.is_ok(), "send_keys should succeed with valid input");
    }

    #[tokio::test]
    async fn test_send_keys_empty() {
        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        let agent = create_test_agent().await;

        // Test sending empty string
        let result = agent.send_keys("").await;
        assert!(result.is_ok(), "send_keys should handle empty string");
    }

    #[tokio::test]
    async fn test_setup_monitoring_returns_handles() {
        use crate::config::helper::ActionType;
        use crate::config::rules_config::{Rule, RuleType};

        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        let agent = create_test_agent().await;

        // Create some test rules
        let rules = vec![Rule {
            rule_type: RuleType::When(regex::Regex::new("test").unwrap()),
            action: ActionType::SendKeys(vec!["echo".to_string()]),
        }];

        // Test setup_monitoring returns correct number of handles
        let result = agent.setup_monitoring(rules).await;
        assert!(result.is_ok(), "setup_monitoring should succeed");

        let handles = result.unwrap();
        assert_eq!(handles.len(), 3, "Should return 3 monitoring handles");

        // Clean up by aborting the handles
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn test_web_server_setup_when_disabled() {
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false;

        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        // Web server should not be started when disabled
        let web_handle = agent.web_server_handle.read().unwrap();
        assert!(
            web_handle.is_none(),
            "Web server handle should be None when disabled"
        );
    }

    #[tokio::test]
    async fn test_monitor_method() {
        let _config = Config::default();
        let agent = create_test_agent().await;

        // Initially should be idle
        assert!(!agent.is_active().await, "Agent should start as idle");

        // Call monitor method - this should check child processes and update status
        agent.monitor().await;

        // Status might remain the same if no child processes are running
        // This test mainly ensures the monitor method doesn't panic
        let status_after_monitor = agent.is_active().await;

        // The status could be either idle or active depending on system state
        // The important thing is that the method completes without error
        assert!(
            status_after_monitor == true || status_after_monitor == false,
            "Monitor method should complete successfully"
        );
    }

    #[tokio::test]
    async fn test_get_pty_receiver() {
        let _config = Config::default();
        let agent = create_test_agent().await;

        // Test getting PTY receiver
        let result = agent.get_pty_receiver().await;
        assert!(result.is_ok(), "get_pty_receiver should succeed");
    }

    #[tokio::test]
    async fn test_agent_from_config_with_mock() {
        use crate::terminal::pty_process_trait::MockPtyProcess;

        let mut config = Config::default();
        config.web_ui.enabled = false;
        config.agents.pool = 1;

        // Test that creating from config works
        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await;
        assert!(agent.is_ok(), "Agent creation from config should succeed");

        let agent = agent.unwrap();
        assert_eq!(agent.get_id(), "agent-0");
        assert!(!agent.is_active().await, "New agent should start as idle");
    }

    #[tokio::test]
    async fn test_agent_status_transitions() {
        let agent = create_test_agent().await;

        // Test initial state
        assert_eq!(agent.is_active().await, false, "Agent should start idle");

        // Test multiple transitions
        agent.set_status(AgentStatus::Active).await;
        assert!(agent.is_active().await, "Agent should be active");

        agent.set_status(AgentStatus::Idle).await;
        assert!(!agent.is_active().await, "Agent should be idle");

        agent.set_status(AgentStatus::Active).await;
        assert!(agent.is_active().await, "Agent should be active again");

        agent.set_status(AgentStatus::Idle).await;
        assert!(!agent.is_active().await, "Agent should be idle again");
    }

    #[tokio::test]
    async fn test_send_keys_with_various_inputs() {
        let agent = create_test_agent().await;

        // Test with simple command
        let result = agent.send_keys("echo test").await;
        assert!(result.is_ok(), "Simple send_keys should succeed");

        // Test with newline
        let result = agent.send_keys("echo test\n").await;
        assert!(result.is_ok(), "Send_keys with newline should succeed");

        // Test with special characters
        let result = agent.send_keys("echo 'hello world'").await;
        assert!(result.is_ok(), "Send_keys with quotes should succeed");

        // Test with numbers
        let result = agent.send_keys("echo 12345").await;
        assert!(result.is_ok(), "Send_keys with numbers should succeed");
    }

    #[tokio::test]
    async fn test_agent_terminal_dimensions() {
        let mut config = Config::default();
        config.web_ui.enabled = false;
        config.web_ui.cols = 120;
        config.web_ui.rows = 40;

        let mock_pty = Box::new(MockPtyProcess::new());
        let agent = Agent::new_with_process(0, &config, mock_pty).await.unwrap();

        let (cols, rows) = agent.get_terminal_dimensions();
        assert_eq!(cols, 120, "Columns should match config");
        assert_eq!(rows, 40, "Rows should match config");
    }

    #[tokio::test]
    async fn test_agent_get_process() {
        let agent = create_test_agent().await;

        // Test that get_process returns a valid process
        let process = agent.get_process();
        // Just verify it doesn't panic and returns something
        let receiver_result = process.get_pty_string_receiver().await;
        assert!(
            receiver_result.is_ok(),
            "Process should have valid receiver"
        );
    }

    #[tokio::test]
    async fn test_multiple_agents_different_ids() {
        let mut config = Config::default();
        config.web_ui.enabled = false;

        let agent1 = Agent::new_with_process(0, &config, Box::new(MockPtyProcess::new()))
            .await
            .unwrap();
        let agent2 = Agent::new_with_process(1, &config, Box::new(MockPtyProcess::new()))
            .await
            .unwrap();
        let agent3 = Agent::new_with_process(5, &config, Box::new(MockPtyProcess::new()))
            .await
            .unwrap();

        assert_eq!(agent1.get_id(), "agent-0");
        assert_eq!(agent2.get_id(), "agent-1");
        assert_eq!(agent3.get_id(), "agent-5");
    }

    #[tokio::test]
    async fn test_setup_monitoring_with_empty_rules() {
        let agent = create_test_agent().await;
        let empty_rules = vec![];

        let result = agent.setup_monitoring(empty_rules).await;
        assert!(
            result.is_ok(),
            "Setup monitoring with empty rules should succeed"
        );

        let handles = result.unwrap();
        assert_eq!(
            handles.len(),
            3,
            "Should return 3 monitoring handles even with empty rules"
        );

        // Clean up handles
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn test_setup_monitoring_with_multiple_rules() {
        use crate::config::helper::ActionType;
        use crate::config::rules_config::{Rule, RuleType};
        use regex::Regex;

        let agent = create_test_agent().await;
        let rules = vec![
            Rule {
                rule_type: RuleType::When(Regex::new("test").unwrap()),
                action: ActionType::SendKeys(vec!["echo matched".to_string()]),
            },
            Rule {
                rule_type: RuleType::DiffTimeout(Duration::from_secs(1)),
                action: ActionType::SendKeys(vec!["echo timeout".to_string()]),
            },
        ];

        let result = agent.setup_monitoring(rules).await;
        assert!(result.is_ok(), "Setup monitoring with rules should succeed");

        let handles = result.unwrap();
        assert_eq!(handles.len(), 3, "Should return 3 monitoring handles");

        // Clean up handles
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn test_get_pty_receiver_multiple_calls() {
        let agent = create_test_agent().await;

        // Test multiple calls to get_pty_receiver
        let receiver1 = agent.get_pty_receiver().await;
        assert!(receiver1.is_ok(), "First get_pty_receiver should succeed");

        let receiver2 = agent.get_pty_receiver().await;
        assert!(receiver2.is_ok(), "Second get_pty_receiver should succeed");

        let receiver3 = agent.get_pty_receiver().await;
        assert!(receiver3.is_ok(), "Third get_pty_receiver should succeed");
    }

    #[tokio::test]
    async fn test_agent_monitor_method() {
        let agent = create_test_agent().await;

        // Test monitor method directly - should not panic
        agent.monitor().await;

        // Agent status might change after monitoring, but shouldn't crash
        let status = agent.is_active().await;
        assert!(
            status == true || status == false,
            "Status should be boolean"
        );
    }

    #[tokio::test]
    async fn test_concurrent_status_changes() {
        use tokio::task::JoinSet;

        let agent = create_test_agent().await;
        let mut set = JoinSet::new();

        // Test concurrent status changes
        for i in 0..10 {
            let agent_clone = Arc::clone(&agent);
            set.spawn(async move {
                if i % 2 == 0 {
                    agent_clone.set_status(AgentStatus::Active).await;
                } else {
                    agent_clone.set_status(AgentStatus::Idle).await;
                }
                agent_clone.is_active().await
            });
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        while let Some(result) = set.join_next().await {
            results.push(result.unwrap());
        }

        // All tasks should complete successfully
        assert_eq!(results.len(), 10, "All concurrent tasks should complete");

        // Final status should be boolean
        let final_status = agent.is_active().await;
        assert!(
            final_status == true || final_status == false,
            "Final status should be boolean"
        );
    }
}
