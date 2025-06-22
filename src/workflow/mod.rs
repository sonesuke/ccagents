pub mod execution;
pub mod hot_reload;
pub mod session;

use crate::agent::Agent;
use crate::ruler::types::ActionType;
use crate::workflow::execution::ActionExecutor;
use crate::workflow::hot_reload::HotReloader;
use crate::workflow::session::SessionStore;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Workflow {
    sessions: Arc<Mutex<SessionStore>>,
    executor: ActionExecutor,
    #[allow(dead_code)]
    hot_reloader: Option<HotReloader>,
    test_mode: bool,
}

#[allow(dead_code)]
impl Workflow {
    pub async fn new(test_mode: bool, rules_path: Option<&str>) -> Result<Self> {
        // Initialize session store with default persistence file
        let session_file = if test_mode {
            PathBuf::from("/tmp/rule-agents-test-sessions.json")
        } else {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("rule-agents")
                .join("sessions.json")
        };

        let mut sessions = SessionStore::new(session_file);
        sessions.load_sessions().await.unwrap_or_else(|e| {
            eprintln!("Warning: Could not load sessions: {}", e);
        });

        let executor = ActionExecutor::new(test_mode);

        // Set up hot reloader if rules path is provided
        let hot_reloader = if let Some(path) = rules_path {
            Some(HotReloader::new(path).await?)
        } else {
            None
        };

        Ok(Workflow {
            sessions: Arc::new(Mutex::new(sessions)),
            executor,
            hot_reloader,
            test_mode,
        })
    }

    pub async fn handle_waiting_state(
        &self,
        agent: &Agent,
        capture: &str,
        action: ActionType,
    ) -> Result<()> {
        println!(
            "Agent {}: Capture \"{}\" → {:?}",
            agent.id(),
            capture,
            action
        );

        // Save session state before executing action (for potential recovery)
        self.save_current_session_state(agent, capture)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Warning: Could not save session state: {}", e);
            });

        // Execute action with graceful error handling
        match self.executor.execute_action(agent, action).await {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("❌ Action execution failed for agent {}: {}", agent.id(), e);

                // Try to recover from the error
                if let Err(recovery_error) = self.handle_interrupted_session(agent, &e).await {
                    eprintln!("❌ Session recovery also failed: {}", recovery_error);
                }

                Err(e)
            }
        }
    }

    async fn save_current_session_state(&self, agent: &Agent, last_command: &str) -> Result<()> {
        if self.test_mode {
            return Ok(());
        }

        // Get current terminal snapshot
        let snapshot = agent
            .take_snapshot()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to take terminal snapshot: {}", e))?;

        // Get current environment variables
        let env_vars = agent
            .get_environment()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get environment variables: {}", e))?;

        // Get current working directory (simplified - in real implementation would be more sophisticated)
        let working_directory = env_vars.get("PWD").unwrap_or(&"/tmp".to_string()).clone();

        let mut sessions = self.sessions.lock().await;
        sessions
            .save_session_state(
                agent.id(),
                &working_directory,
                snapshot,
                env_vars,
                Some(last_command.to_string()),
            )
            .await?;

        Ok(())
    }

    async fn handle_interrupted_session(&self, agent: &Agent, error: &anyhow::Error) -> Result<()> {
        println!(
            "🔧 Attempting to recover from interrupted session for agent {}",
            agent.id()
        );

        if self.test_mode {
            println!("ℹ️ Test mode: simulating session recovery");
            return Ok(());
        }

        // Check if backend is still available
        if !agent.is_available().await {
            return Err(anyhow::anyhow!("Agent is no longer available"));
        }

        // Try to send Ctrl+C to cancel any hanging processes
        println!("🛑 Sending interrupt signal...");
        if let Err(e) = agent.send_keys("^C").await {
            eprintln!("Failed to send interrupt: {}", e);
        }

        // Wait a moment for processes to terminate
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Try to reset the terminal state
        println!("🔄 Resetting terminal state...");

        // Send a few newlines to get to a clean prompt
        for _ in 0..3 {
            if let Err(e) = agent.send_keys("\r").await {
                eprintln!("Failed to send newline: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        // Try to execute a simple command to verify the terminal is responsive
        println!("🧪 Testing terminal responsiveness...");
        if agent.send_keys("echo 'Terminal recovered'").await.is_ok() {
            agent.send_keys("\r").await.ok();
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Take a snapshot to see if we got output
            if let Ok(snapshot) = agent.take_snapshot().await {
                if snapshot.content.contains("Terminal recovered") {
                    println!("✅ Terminal session recovered successfully");

                    // Save the recovery state
                    self.save_current_session_state(agent, "session_recovered")
                        .await
                        .ok();
                    return Ok(());
                }
            }
        }

        // If we can't recover, mark this in the session
        println!("⚠️ Terminal recovery incomplete - session may need manual intervention");

        // Update session state to indicate it needs attention
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_session_mut(agent.id()) {
            session.last_command = Some(format!("ERROR: {}", error));
        }
        drop(sessions);

        Err(anyhow::anyhow!("Terminal session recovery incomplete"))
    }

    pub async fn execute_resume_command(&self, agent: &Agent) -> Result<()> {
        println!("▶️ Executing resume command for agent {}", agent.id());

        // In test mode, just simulate resume
        if self.test_mode {
            println!("ℹ️ Test mode: resume command simulated");
            return Ok(());
        }

        let session_state = {
            let sessions = self.sessions.lock().await;
            sessions.get_latest_session_for_agent(agent.id()).cloned()
        };

        // Try to find the most recent session for this agent
        if let Some(session_state) = session_state {
            println!("📄 Found session state for agent {}", agent.id());
            println!("  - Working directory: {}", session_state.working_directory);
            println!("  - Last command: {:?}", session_state.last_command);
            println!("  - Session timestamp: {}", session_state.timestamp);

            // Restore session state
            self.restore_session_state(agent, &session_state).await?;

            println!("✅ Session restored successfully for agent {}", agent.id());
        } else {
            println!("⚠️ No previous session found for agent {}", agent.id());
            println!("   Starting fresh terminal session");

            // Start a fresh session since no previous state exists
            self.start_fresh_session(agent).await?;
        }

        Ok(())
    }

    async fn restore_session_state(
        &self,
        agent: &Agent,
        session_state: &crate::workflow::session::SessionState,
    ) -> Result<()> {
        println!("🔄 Restoring session state for agent {}", agent.id());

        // Restore working directory
        println!(
            "📁 Restoring working directory: {}",
            session_state.working_directory
        );
        agent
            .set_working_directory(&session_state.working_directory)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to restore working directory: {}", e))?;

        // Restore environment variables (for HT backend, this would involve setting them)
        for (key, value) in &session_state.environment_vars {
            if key != "PWD" && key != "HOME" && key != "USER" {
                // Skip system variables
                let env_cmd = format!("export {}={}", key, value);
                agent.send_keys(&env_cmd).await.ok(); // Best effort
                agent.send_keys("\r").await.ok();
            }
        }

        // If there was a last command, show it as a suggestion but don't execute it
        if let Some(ref last_cmd) = session_state.last_command {
            println!("💡 Last command was: {}", last_cmd);
            println!("   You can press Up arrow or retype to continue");
        }

        // Show current terminal content summary
        let content_lines = session_state.terminal_snapshot.content.lines().count();
        println!(
            "📺 Restored terminal session with {} lines of content",
            content_lines
        );

        Ok(())
    }

    async fn start_fresh_session(&self, agent: &Agent) -> Result<()> {
        println!("🆕 Starting fresh session for agent {}", agent.id());

        // Clear terminal and show welcome message
        agent.send_keys("clear").await.ok();
        agent.send_keys("\r").await.ok();

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let welcome_msg = format!("echo '🤖 Agent {} session started'", agent.id());
        agent.send_keys(&welcome_msg).await.ok();
        agent.send_keys("\r").await.ok();

        println!("✅ Fresh session initialized for agent {}", agent.id());
        Ok(())
    }

    pub async fn cleanup_old_sessions(&self, max_age_hours: u64) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        sessions.cleanup_old_sessions(max_age_hours).await
    }

    pub async fn list_sessions(&self) -> Vec<crate::workflow::session::SessionState> {
        let sessions = self.sessions.lock().await;
        sessions.list_sessions().into_iter().cloned().collect()
    }

    pub async fn remove_session(&self, agent_id: &str) -> Result<bool> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove_session(agent_id).await
    }

    pub async fn force_cleanup_agent(&self, agent: &Agent) -> Result<()> {
        println!("🧹 Force cleaning up agent session: {}", agent.id());

        if !self.test_mode {
            // Send multiple interrupt signals
            for _ in 0..3 {
                agent.send_keys("^C").await.ok();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Clear the terminal
            agent.send_keys("clear").await.ok();
            agent.send_keys("\r").await.ok();
        }

        // Remove the session from persistence
        self.remove_session(agent.id()).await.ok();

        println!("✅ Agent {} cleaned up", agent.id());
        Ok(())
    }

    pub async fn emergency_stop_all(&self, agents: &[&Agent]) -> Result<()> {
        println!("🚨 Emergency stop: cleaning up all sessions");

        if !self.test_mode {
            for agent in agents {
                // Send multiple interrupt signals
                for _ in 0..5 {
                    agent.send_keys("^C").await.ok();
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }

                // Note: Agent cleanup is handled automatically by OS when parent process exits
                println!("🛑 Stopped agent {}", agent.id());
            }
        }

        // Clear all sessions
        let mut sessions = self.sessions.lock().await;
        sessions.clear_all_sessions();
        sessions.save_sessions().await.ok();

        println!("🛑 Emergency stop completed");
        Ok(())
    }
}
