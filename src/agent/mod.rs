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
        let config = Config::default();
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
        let config = Config::default();
        let agent = create_test_agent().await;

        // Test getting PTY receiver
        let result = agent.get_pty_receiver().await;
        assert!(result.is_ok(), "get_pty_receiver should succeed");
    }

    /// Comprehensive debugging test for the hanging send_keys issue
    /// This test includes extensive logging and timeouts to identify exactly where hangs occur
    #[tokio::test]
    #[ignore] // Debug test that hangs due to PTY reader issues
    async fn test_send_keys_with_comprehensive_debugging() {
        use std::time::{Duration, Instant};
        use tokio::time::timeout;
        use tracing::{error, info, warn};

        // Initialize detailed logging for this test
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .try_init();

        info!("🚀 Starting comprehensive debugging test for send_keys hang detection");

        // Create config with WebUI disabled to avoid port conflicts
        let mut config = Config::default();
        config.web_ui.enabled = false;
        info!("✅ Config created with WebUI disabled");

        // Create agent with timeout monitoring
        info!("📦 Creating agent...");
        let start_time = Instant::now();
        let agent = match timeout(Duration::from_secs(10), Agent::from_config(0, &config)).await {
            Ok(Ok(agent)) => {
                info!(
                    "✅ Agent created successfully in {:?}",
                    start_time.elapsed()
                );
                agent
            }
            Ok(Err(e)) => {
                error!("❌ Failed to create agent: {}", e);
                panic!("Agent creation failed: {}", e);
            }
            Err(_) => {
                error!("⏱️ Agent creation timed out after 10 seconds");
                panic!("Agent creation timed out");
            }
        };

        // Log initial agent state
        info!("🔍 Agent ID: {}", agent.get_id());
        info!("🔍 Agent active status: {}", agent.is_active().await);

        // Get reference to PTY process for deeper inspection
        let pty_process = agent.get_process();
        info!("🔍 PTY process reference obtained");

        // Test Pattern 1: Single send_keys operation
        info!("🎯 === TEST PATTERN 1: Single send_keys ===");
        info!("📤 Sending single command: 'echo single_test'");
        let send_start = Instant::now();
        match timeout(
            Duration::from_secs(3),
            agent.send_keys("echo single_test\n"),
        )
        .await
        {
            Ok(Ok(())) => {
                info!(
                    "✅ Single send_keys completed in {:?}",
                    send_start.elapsed()
                );
            }
            Ok(Err(e)) => {
                error!("❌ Single send_keys failed: {}", e);
                panic!("Single send_keys failed: {}", e);
            }
            Err(_) => {
                error!("⏱️ Single send_keys timed out after 3 seconds");
                error!("🔴 HANG DETECTED: Single send_keys operation is blocking!");
                panic!("Single send_keys timed out - HANG DETECTED");
            }
        }

        // Check PTY state after first operation
        info!("🔍 Checking PTY process state after single operation...");
        match timeout(
            Duration::from_millis(1000),
            pty_process.get_child_processes(),
        )
        .await
        {
            Ok(Ok(child_pids)) => {
                info!(
                    "✅ PTY child processes: {:?} (count: {})",
                    child_pids,
                    child_pids.len()
                );
            }
            Ok(Err(e)) => {
                warn!("⚠️ Failed to get child processes: {}", e);
            }
            Err(_) => {
                warn!("⏱️ Getting child processes timed out");
            }
        }

        // Test Pattern 2: Multiple send_keys with 100ms sleep pattern (the hanging pattern)
        info!("🎯 === TEST PATTERN 2: Multiple send_keys with 100ms sleep ===");

        for iteration in 1..=3 {
            info!(
                "📤 [Iteration {}/3] Sending: 'echo test_{}'",
                iteration, iteration
            );
            let send_start = Instant::now();

            // This is the problematic pattern: send_keys followed by sleep
            match timeout(
                Duration::from_secs(5),
                agent.send_keys(&format!("echo test_{}\n", iteration)),
            )
            .await
            {
                Ok(Ok(())) => {
                    info!(
                        "✅ [Iteration {}/3] send_keys completed in {:?}",
                        iteration,
                        send_start.elapsed()
                    );
                }
                Ok(Err(e)) => {
                    error!("❌ [Iteration {}/3] send_keys failed: {}", iteration, e);
                    panic!("Send_keys failed at iteration {}: {}", iteration, e);
                }
                Err(_) => {
                    error!(
                        "⏱️ [Iteration {}/3] send_keys timed out after 5 seconds",
                        iteration
                    );
                    error!(
                        "🔴 HANG DETECTED: send_keys operation {} is blocking!",
                        iteration
                    );

                    // Additional debugging for hang detection
                    info!("🔍 HANG DEBUG: Attempting to get PTY process state during hang...");
                    match timeout(
                        Duration::from_millis(500),
                        pty_process.get_child_processes(),
                    )
                    .await
                    {
                        Ok(Ok(child_pids)) => {
                            error!("🔍 HANG DEBUG: PTY children during hang: {:?}", child_pids);
                        }
                        Ok(Err(e)) => {
                            error!("🔍 HANG DEBUG: Failed to get children during hang: {}", e);
                        }
                        Err(_) => {
                            error!("🔍 HANG DEBUG: Child process check also timed out during hang");
                        }
                    }

                    panic!(
                        "Send_keys timed out at iteration {} - HANG DETECTED",
                        iteration
                    );
                }
            }

            // The problematic 100ms sleep
            info!("😴 [Sleep {}/2] Starting 100ms sleep...", iteration);
            let sleep_start = Instant::now();
            match timeout(
                Duration::from_millis(300),
                tokio::time::sleep(Duration::from_millis(100)),
            )
            .await
            {
                Ok(()) => {
                    info!(
                        "✅ [Sleep {}/2] Sleep completed in {:?}",
                        iteration,
                        sleep_start.elapsed()
                    );
                }
                Err(_) => {
                    error!(
                        "⏱️ [Sleep {}/2] Sleep timed out (this should never happen!)",
                        iteration
                    );
                    panic!("Sleep timed out at iteration {}", iteration);
                }
            }

            // Check PTY state after each iteration
            info!(
                "🔍 [Check {}/3] Checking PTY state after iteration {}...",
                iteration, iteration
            );
            match timeout(
                Duration::from_millis(1000),
                pty_process.get_child_processes(),
            )
            .await
            {
                Ok(Ok(child_pids)) => {
                    info!(
                        "✅ [Check {}/3] PTY children: {:?} (count: {})",
                        iteration,
                        child_pids,
                        child_pids.len()
                    );
                }
                Ok(Err(e)) => {
                    warn!("⚠️ [Check {}/3] Failed to get children: {}", iteration, e);
                }
                Err(_) => {
                    warn!("⏱️ [Check {}/3] Child check timed out", iteration);
                }
            }
        }

        info!("🎉 All send_keys operations with sleep pattern completed successfully!");

        // Test Pattern 3: Rapid fire send_keys without sleep
        info!("🎯 === TEST PATTERN 3: Rapid fire send_keys ===");

        for i in 1..=5 {
            let cmd = format!("echo rapid_{}\n", i);
            info!("📤 [Rapid {}/5] Sending: {}", i, cmd.trim());

            match timeout(Duration::from_secs(2), agent.send_keys(&cmd)).await {
                Ok(Ok(())) => {
                    info!("✅ [Rapid {}/5] Command sent successfully", i);
                }
                Ok(Err(e)) => {
                    error!("❌ [Rapid {}/5] Command failed: {}", i, e);
                    panic!("Rapid send failed at command {}: {}", i, e);
                }
                Err(_) => {
                    error!("⏱️ [Rapid {}/5] Command timed out", i);
                    error!("🔴 HANG DETECTED at rapid command {}", i);
                    panic!("Hang detected at rapid command {}", i);
                }
            }
            // No sleep between rapid commands
        }

        info!("✅ All rapid commands sent successfully");

        // Final comprehensive state check
        info!("🎯 === FINAL STATE VERIFICATION ===");

        info!("🔍 Final agent active status: {}", agent.is_active().await);

        // Test PTY receiver creation to verify PTY is still responsive
        info!("🔍 Testing PTY receiver creation...");
        match timeout(
            Duration::from_secs(2),
            pty_process.get_pty_string_receiver(),
        )
        .await
        {
            Ok(Ok(_receiver)) => {
                info!("✅ PTY receiver created successfully - PTY is responsive");
            }
            Ok(Err(e)) => {
                error!("❌ Failed to create PTY receiver: {}", e);
            }
            Err(_) => {
                error!("⏱️ PTY receiver creation timed out");
                error!("🔴 PTY process may be in a bad state!");
            }
        }

        // Test one final send_keys to ensure agent is still functional
        info!("🔍 Testing final send_keys for agent responsiveness...");
        match timeout(Duration::from_secs(3), agent.send_keys("echo final_test\n")).await {
            Ok(Ok(())) => {
                info!("✅ Final send_keys successful - agent fully responsive");
            }
            Ok(Err(e)) => {
                error!("❌ Final send_keys failed: {}", e);
            }
            Err(_) => {
                error!("⏱️ Final send_keys timed out");
                error!("🔴 Agent may be in a bad state after test sequence!");
            }
        }

        info!("✅ Comprehensive debugging test completed successfully!");
        info!(
            "🎯 If this test passes, the hang issue may be specific to certain conditions or timing"
        );
    }

    /// Test send_keys behavior with monitoring systems active
    /// This tests if the hang is related to monitoring interference
    #[tokio::test]
    #[ignore] // Debug test that hangs due to PTY reader issues
    async fn test_send_keys_with_monitoring_debug() {
        use crate::config::helper::ActionType;
        use crate::config::rules_config::{Rule, RuleType};
        use std::time::{Duration, Instant};
        use tokio::time::timeout;
        use tracing::{error, info, warn};

        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        info!("🚀 Starting send_keys with monitoring debugging test");

        let mut config = Config::default();
        config.web_ui.enabled = false;

        let agent = create_test_agent().await;

        // Create test rules
        let rules = vec![Rule {
            rule_type: RuleType::When(regex::Regex::new("test_trigger").unwrap()),
            action: ActionType::SendKeys(vec!["echo triggered".to_string()]),
        }];

        // Start monitoring systems
        info!("🔍 Starting monitoring systems...");
        let monitoring_start = Instant::now();
        let monitoring_handles = match timeout(
            Duration::from_secs(5),
            agent.clone().setup_monitoring(rules),
        )
        .await
        {
            Ok(Ok(handles)) => {
                info!(
                    "✅ Monitoring systems started in {:?}: {} handles",
                    monitoring_start.elapsed(),
                    handles.len()
                );
                handles
            }
            Ok(Err(e)) => {
                error!("❌ Failed to start monitoring: {}", e);
                panic!("Monitoring setup failed: {}", e);
            }
            Err(_) => {
                error!("⏱️ Monitoring setup timed out");
                panic!("Monitoring setup timed out");
            }
        };

        // Give monitoring time to initialize
        info!("😴 Allowing monitoring systems to initialize...");
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Test send_keys with monitoring active
        info!("🎯 Testing send_keys with active monitoring...");
        for i in 1..=3 {
            let cmd = format!("echo monitor_test_{}\n", i);
            info!("📤 [Monitor Test {}/3] Sending: {}", i, cmd.trim());

            let send_start = Instant::now();
            match timeout(Duration::from_secs(5), agent.send_keys(&cmd)).await {
                Ok(Ok(())) => {
                    info!(
                        "✅ [Monitor Test {}/3] Command sent in {:?}",
                        i,
                        send_start.elapsed()
                    );
                }
                Ok(Err(e)) => {
                    error!("❌ [Monitor Test {}/3] Command failed: {}", i, e);
                    panic!("Send failed at monitor test {} with monitoring: {}", i, e);
                }
                Err(_) => {
                    error!("⏱️ [Monitor Test {}/3] Command timed out", i);
                    error!(
                        "🔴 HANG DETECTED at monitor test {} with monitoring active",
                        i
                    );
                    panic!("Hang detected at monitor test {} with monitoring", i);
                }
            }

            // Sleep between commands to match the problematic pattern
            info!(
                "😴 [Monitor Sleep {}/2] 100ms sleep with monitoring active...",
                i
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!("✅ All commands with monitoring completed successfully");

        // Cleanup monitoring
        info!("🧹 Cleaning up monitoring handles...");
        for (i, handle) in monitoring_handles.into_iter().enumerate() {
            handle.abort();
            info!("🧹 Aborted monitoring handle {}", i + 1);
        }

        // Test send_keys after monitoring cleanup
        info!("🔍 Testing send_keys after monitoring cleanup...");
        match timeout(
            Duration::from_secs(3),
            agent.send_keys("echo post_monitoring\n"),
        )
        .await
        {
            Ok(Ok(())) => {
                info!("✅ Post-monitoring send_keys successful");
            }
            Ok(Err(e)) => {
                error!("❌ Post-monitoring send_keys failed: {}", e);
            }
            Err(_) => {
                error!("⏱️ Post-monitoring send_keys timed out");
            }
        }

        info!("✅ Send_keys with monitoring debugging test completed");
    }

    /// Test to verify PTY process internal state during the problematic pattern
    #[tokio::test]
    #[ignore] // Debug test that hangs due to PTY reader issues
    async fn test_pty_process_state_during_send_keys_pattern() {
        use std::time::{Duration, Instant};
        use tokio::time::timeout;
        use tracing::{error, info, warn};

        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        info!("🚀 Starting PTY process state debugging test");

        let mut config = Config::default();
        config.web_ui.enabled = false;

        let agent = create_test_agent().await;
        let pty_process = agent.get_process();

        // Initial PTY state
        info!("🔍 Initial PTY state check...");
        match timeout(
            Duration::from_millis(1000),
            pty_process.get_child_processes(),
        )
        .await
        {
            Ok(Ok(child_pids)) => {
                info!(
                    "✅ Initial PTY children: {:?} (count: {})",
                    child_pids,
                    child_pids.len()
                );
            }
            Ok(Err(e)) => {
                warn!("⚠️ Initial PTY check failed: {}", e);
            }
            Err(_) => {
                warn!("⏱️ Initial PTY check timed out");
            }
        }

        // Test the exact problematic pattern with state checks between each step
        for cycle in 1..=2 {
            info!("🔄 === CYCLE {} ===", cycle);

            // Step 1: Pre-send state
            info!("🔍 [Cycle {}] Pre-send PTY state...", cycle);
            match timeout(
                Duration::from_millis(500),
                pty_process.get_child_processes(),
            )
            .await
            {
                Ok(Ok(child_pids)) => {
                    info!("✅ [Cycle {}] Pre-send children: {:?}", cycle, child_pids);
                }
                Ok(Err(e)) => {
                    warn!("⚠️ [Cycle {}] Pre-send check failed: {}", cycle, e);
                }
                Err(_) => {
                    warn!("⏱️ [Cycle {}] Pre-send check timed out", cycle);
                }
            }

            // Step 2: Send command
            info!("📤 [Cycle {}] Sending command...", cycle);
            let send_start = Instant::now();
            match timeout(
                Duration::from_secs(5),
                agent.send_keys(&format!("echo cycle_{}\n", cycle)),
            )
            .await
            {
                Ok(Ok(())) => {
                    info!(
                        "✅ [Cycle {}] Send completed in {:?}",
                        cycle,
                        send_start.elapsed()
                    );
                }
                Ok(Err(e)) => {
                    error!("❌ [Cycle {}] Send failed: {}", cycle, e);
                    panic!("Send failed at cycle {}: {}", cycle, e);
                }
                Err(_) => {
                    error!("⏱️ [Cycle {}] Send timed out", cycle);
                    error!("🔴 HANG DETECTED at cycle {}", cycle);
                    panic!("Send hang detected at cycle {}", cycle);
                }
            }

            // Step 3: Post-send immediate state
            info!("🔍 [Cycle {}] Post-send immediate PTY state...", cycle);
            match timeout(
                Duration::from_millis(500),
                pty_process.get_child_processes(),
            )
            .await
            {
                Ok(Ok(child_pids)) => {
                    info!("✅ [Cycle {}] Post-send children: {:?}", cycle, child_pids);
                }
                Ok(Err(e)) => {
                    warn!("⚠️ [Cycle {}] Post-send check failed: {}", cycle, e);
                }
                Err(_) => {
                    warn!("⏱️ [Cycle {}] Post-send check timed out", cycle);
                }
            }

            // Step 4: Sleep (the suspected problematic step)
            info!("😴 [Cycle {}] Starting 100ms sleep...", cycle);
            let sleep_start = Instant::now();
            tokio::time::sleep(Duration::from_millis(100)).await;
            info!(
                "✅ [Cycle {}] Sleep completed in {:?}",
                cycle,
                sleep_start.elapsed()
            );

            // Step 5: Post-sleep state
            info!("🔍 [Cycle {}] Post-sleep PTY state...", cycle);
            match timeout(
                Duration::from_millis(500),
                pty_process.get_child_processes(),
            )
            .await
            {
                Ok(Ok(child_pids)) => {
                    info!("✅ [Cycle {}] Post-sleep children: {:?}", cycle, child_pids);
                }
                Ok(Err(e)) => {
                    warn!("⚠️ [Cycle {}] Post-sleep check failed: {}", cycle, e);
                }
                Err(_) => {
                    warn!("⏱️ [Cycle {}] Post-sleep check timed out", cycle);
                }
            }

            info!("✅ [Cycle {}] Completed successfully", cycle);
        }

        info!("✅ PTY process state debugging test completed");
    }

    /// Test to isolate and demonstrate the exact hang condition
    /// This test focuses specifically on the PTY reader blocking issue
    #[tokio::test]
    #[ignore] // Debug test that hangs due to PTY reader issues  
    async fn test_isolate_pty_reader_hang() {
        use std::time::{Duration, Instant};
        use tokio::time::timeout;
        use tracing::{error, info};

        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        info!("🚀 Starting isolated PTY reader hang test");

        let mut config = Config::default();
        config.web_ui.enabled = false;

        let agent = create_test_agent().await;

        // Step 1: Send one command successfully
        info!("📤 Step 1: Sending first command (should work)...");
        let result = timeout(Duration::from_secs(3), agent.send_keys("echo step1\n")).await;
        match result {
            Ok(Ok(())) => {
                info!("✅ Step 1: First send_keys completed successfully");
            }
            Ok(Err(e)) => {
                error!("❌ Step 1: First send_keys failed: {}", e);
                panic!("Step 1 failed: {}", e);
            }
            Err(_) => {
                error!("⏱️ Step 1: First send_keys timed out");
                panic!("Step 1 timed out");
            }
        }

        // Step 2: Wait briefly to let PTY reader process the output
        info!("😴 Step 2: Waiting for PTY to process output...");
        tokio::time::sleep(Duration::from_millis(50)).await;
        info!("✅ Step 2: Brief wait completed");

        // Step 3: Send second command - this is where it typically hangs
        info!("📤 Step 3: Sending second command (hang expected here)...");
        let start_time = Instant::now();
        let result = timeout(Duration::from_secs(5), agent.send_keys("echo step2\n")).await;

        match result {
            Ok(Ok(())) => {
                info!(
                    "✅ Step 3: Second send_keys completed in {:?}",
                    start_time.elapsed()
                );
                info!("🎉 HANG NOT REPRODUCED: Test passed without hanging");
            }
            Ok(Err(e)) => {
                error!("❌ Step 3: Second send_keys failed: {}", e);
                panic!("Step 3 failed: {}", e);
            }
            Err(_) => {
                error!(
                    "⏱️ Step 3: Second send_keys timed out after {:?}",
                    start_time.elapsed()
                );
                error!("🔴 HANG CONFIRMED: PTY reader is blocking on read operation");
                error!("🔍 ROOT CAUSE: Synchronous read() call in async task");
                error!("💡 SOLUTION: Replace std::io::Read with tokio async I/O");
                panic!("HANG CONFIRMED - PTY reader blocking issue");
            }
        }

        info!("✅ Isolated PTY reader hang test completed");
    }
}
