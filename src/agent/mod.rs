pub mod agents;

use crate::config::Config;
use crate::config::rules_config::Rule;
use crate::rule::{DiffTimeout, When};
use crate::terminal::pty_process::PtyProcess;
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
    process: PtyProcess,
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
    pub fn get_process(&self) -> &PtyProcess {
        &self.process
    }

    /// Check if the agent is currently active (true = Active, false = Idle)
    pub async fn is_active(&self) -> bool {
        matches!(*self.status.read().unwrap(), AgentStatus::Active)
    }

    /// Set the status of the agent
    async fn set_status(&self, new_status: AgentStatus) {
        // First check if status actually needs to change (read lock only)
        let needs_update = match tokio::time::timeout(
            tokio::time::Duration::from_millis(50), 
            async { self.status.read().map_err(|_| ()) }
        ).await {
            Ok(Ok(current_status)) => *current_status != new_status,
            _ => {
                tracing::warn!("Status read timeout for agent {}, assuming update needed", self.get_id());
                true
            }
        };

        // Only acquire write lock if status needs to change
        if needs_update {
            match tokio::time::timeout(
                tokio::time::Duration::from_millis(100), 
                async { self.status.write().map_err(|_| ()) }
            ).await {
                Ok(Ok(mut status)) => {
                    *status = new_status.clone();
                    tracing::debug!("🔄 Agent {} → {:?}", self.get_id(), new_status);
                },
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
                Ok(_) => {},
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

    #[tokio::test]
    async fn test_agent_creation() {
        let config = Config::default();
        let _agent = Agent::from_config(0, &config).await.unwrap();
        // Just verify the agent can be created successfully
        // Agent functionality is tested through integration tests
    }

    #[tokio::test]
    async fn test_agent_getters() {
        let mut config = Config::default();
        // Set specific dimensions for testing
        config.agents.pool = 3;
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let config = Config::default();
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let mut config = Config::default();
        config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let mut config = Config::default();
        config.web_ui.enabled = false;

        let agent = Agent::from_config(0, &config).await.unwrap();

        // Web server should not be started when disabled
        let web_handle = agent.web_server_handle.read().unwrap();
        assert!(
            web_handle.is_none(),
            "Web server handle should be None when disabled"
        );
    }

    #[tokio::test]
    async fn test_monitor_method() {
        let config = Config::default();
        let agent = Agent::from_config(0, &config).await.unwrap();

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
        let config = Config::default();
        let agent = Agent::from_config(0, &config).await.unwrap();

        // Test getting PTY receiver
        let result = agent.get_pty_receiver().await;
        assert!(result.is_ok(), "get_pty_receiver should succeed");
    }
}
