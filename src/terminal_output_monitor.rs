use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

use crate::ht_client::{HtClient, HtClientError, HtEvent, TerminalSnapshot};

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("HT client error: {0}")]
    HtClientError(#[from] HtClientError),
    #[error("Event subscription failed: {0}")]
    SubscriptionError(String),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Monitor timeout: {0}")]
    Timeout(String),
    #[error("Monitor not running")]
    NotRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Shell prompt is displayed
    Idle,
    /// Command is running but terminal output has no changes over time
    Wait,
    /// Command is running and terminal output is continuously changing
    Active,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: AgentState,
    pub to: AgentState,
    pub timestamp: Instant,
    pub snapshot: TerminalSnapshot,
}

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Interval between terminal snapshots for change detection
    pub change_detection_interval: Duration,
    /// How long to wait without output changes before transitioning to Wait state
    pub wait_timeout: Duration,
    /// Maximum time to stay in Wait state before considering command stuck
    pub stuck_timeout: Duration,
    /// Shell prompt patterns for Idle state detection
    pub prompt_patterns: Vec<String>,
    /// Number of snapshots to keep in history for comparison
    pub snapshot_history_size: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            change_detection_interval: Duration::from_millis(500),
            wait_timeout: Duration::from_secs(2),
            stuck_timeout: Duration::from_secs(30),
            prompt_patterns: vec![
                r".*@.*:\S*\$ $".to_string(), // bash prompt: user@hostname:/path$
                r".*# $".to_string(),         // root prompt: #
                r".*> $".to_string(),         // other shell prompts: >
                r"\$ $".to_string(),          // simple $ prompt
                r"# $".to_string(),           // simple # prompt
            ],
            snapshot_history_size: 10,
        }
    }
}

pub struct TerminalOutputMonitor {
    ht_client: Arc<HtClient>,
    config: MonitorConfig,
    event_receiver: Option<mpsc::UnboundedReceiver<HtEvent>>,
    state_tx: Option<mpsc::UnboundedSender<StateTransition>>,
    current_state: AgentState,
    previous_snapshots: VecDeque<TerminalSnapshot>,
    prompt_patterns: Vec<Regex>,
    last_output_change: Instant,
    running: bool,
}

impl TerminalOutputMonitor {
    /// Create a new TerminalOutputMonitor with proper event subscription
    pub async fn new(ht_client: Arc<HtClient>) -> Result<Self, MonitorError> {
        Self::with_config(ht_client, MonitorConfig::default()).await
    }

    /// Create a new TerminalOutputMonitor with custom configuration
    pub async fn with_config(
        ht_client: Arc<HtClient>,
        config: MonitorConfig,
    ) -> Result<Self, MonitorError> {
        // Subscribe to terminal output events first (critical for take_snapshot to work)
        let events = vec!["terminalOutput".to_string()];
        let event_receiver = ht_client.subscribe_to_events(events).await?;

        // Compile regex patterns
        let prompt_patterns = config
            .prompt_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            ht_client,
            config,
            event_receiver: Some(event_receiver),
            state_tx: None,
            current_state: AgentState::Idle, // Start in Idle state
            previous_snapshots: VecDeque::new(),
            prompt_patterns,
            last_output_change: Instant::now(),
            running: false,
        })
    }

    /// Start monitoring terminal output and return a receiver for state transitions
    pub fn start_monitoring(&mut self) -> mpsc::UnboundedReceiver<StateTransition> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.state_tx = Some(tx);
        self.running = true;
        self.last_output_change = Instant::now();

        info!("Terminal output monitoring started");
        rx
    }

    /// Stop monitoring terminal output
    pub fn stop_monitoring(&mut self) {
        self.running = false;
        self.state_tx = None;
        info!("Terminal output monitoring stopped");
    }

    /// Check if monitoring is currently running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get the current agent state
    pub fn current_state(&self) -> &AgentState {
        &self.current_state
    }

    /// Main monitoring loop - should be run in a separate task
    pub async fn run_monitoring_loop(&mut self) -> Result<(), MonitorError> {
        if !self.running {
            return Err(MonitorError::NotRunning);
        }

        let mut event_receiver = self.event_receiver.take().ok_or_else(|| {
            MonitorError::SubscriptionError("No event receiver available".to_string())
        })?;

        let mut change_interval = interval(self.config.change_detection_interval);

        info!("Starting terminal output monitoring loop");

        loop {
            if !self.running {
                break;
            }

            tokio::select! {
                // Handle terminal output events
                event = event_receiver.recv() => {
                    if let Some(event) = event {
                        self.handle_terminal_event(event).await?;
                    } else {
                        warn!("Event receiver closed, stopping monitoring");
                        break;
                    }
                }

                // Periodic snapshot comparison for change detection
                _ = change_interval.tick() => {
                    self.check_for_output_changes().await?;
                }
            }
        }

        self.event_receiver = Some(event_receiver);
        Ok(())
    }

    /// Handle terminal output events
    async fn handle_terminal_event(&mut self, event: HtEvent) -> Result<(), MonitorError> {
        match event {
            HtEvent::TerminalOutput { data: _ } => {
                // Terminal output received - mark as activity
                self.last_output_change = Instant::now();

                // If we were in Wait state, transition to Active
                if self.current_state == AgentState::Wait {
                    self.transition_to_state(AgentState::Active).await?;
                }
            }
            HtEvent::ProcessExit { code: _ } => {
                // Process exited - likely back to shell prompt
                debug!("Process exit detected, checking for idle state");
                tokio::time::sleep(Duration::from_millis(100)).await; // Brief delay for prompt to appear
                self.check_for_idle_state().await?;
            }
            _ => {
                // Other events not relevant for state detection
            }
        }
        Ok(())
    }

    /// Check for output changes by comparing terminal snapshots
    async fn check_for_output_changes(&mut self) -> Result<(), MonitorError> {
        // Take a new snapshot
        let snapshot = timeout(Duration::from_secs(5), self.ht_client.take_snapshot())
            .await
            .map_err(|_| MonitorError::Timeout("Snapshot timeout".to_string()))?
            .map_err(MonitorError::HtClientError)?;

        // Check if we're in idle state (shell prompt detected)
        if self.is_idle_state(&snapshot) {
            if self.current_state != AgentState::Idle {
                self.transition_to_state(AgentState::Idle).await?;
            }
        } else {
            // Not idle - check for output changes
            let has_changes = self.has_output_changed(&snapshot);

            if has_changes {
                self.last_output_change = Instant::now();
                if self.current_state != AgentState::Active {
                    self.transition_to_state(AgentState::Active).await?;
                }
            } else {
                // No changes detected
                let time_since_change = self.last_output_change.elapsed();

                if time_since_change >= self.config.wait_timeout {
                    if self.current_state != AgentState::Wait {
                        self.transition_to_state(AgentState::Wait).await?;
                    }

                    // Check for stuck command timeout
                    if time_since_change >= self.config.stuck_timeout {
                        warn!(
                            "Command appears stuck - no output changes for {:?}",
                            time_since_change
                        );
                    }
                }
            }
        }

        // Add snapshot to history
        self.add_snapshot_to_history(snapshot);

        Ok(())
    }

    /// Check specifically for idle state after process exit
    async fn check_for_idle_state(&mut self) -> Result<(), MonitorError> {
        let snapshot = self.ht_client.take_snapshot().await?;

        if self.is_idle_state(&snapshot) {
            self.transition_to_state(AgentState::Idle).await?;
        }

        self.add_snapshot_to_history(snapshot);
        Ok(())
    }

    /// Check if terminal is in idle state (shell prompt detected)
    fn is_idle_state(&self, snapshot: &TerminalSnapshot) -> bool {
        // Get the last line of terminal content
        let lines: Vec<&str> = snapshot.content.lines().collect();
        if let Some(last_line) = lines.last() {
            // Check if last line matches any shell prompt pattern
            for pattern in &self.prompt_patterns {
                if pattern.is_match(last_line) {
                    debug!("Shell prompt detected: '{}'", last_line);
                    return true;
                }
            }
        }
        false
    }

    /// Check if terminal output has changed compared to previous snapshots
    fn has_output_changed(&self, current_snapshot: &TerminalSnapshot) -> bool {
        if let Some(previous_snapshot) = self.previous_snapshots.back() {
            // Compare content, but also consider cursor position changes
            let content_changed = current_snapshot.content != previous_snapshot.content;
            let cursor_changed = current_snapshot.cursor_x != previous_snapshot.cursor_x
                || current_snapshot.cursor_y != previous_snapshot.cursor_y;

            content_changed || cursor_changed
        } else {
            // No previous snapshot - consider this as change
            true
        }
    }

    /// Add snapshot to history, maintaining the configured history size
    fn add_snapshot_to_history(&mut self, snapshot: TerminalSnapshot) {
        self.previous_snapshots.push_back(snapshot);

        // Trim history to configured size
        while self.previous_snapshots.len() > self.config.snapshot_history_size {
            self.previous_snapshots.pop_front();
        }
    }

    /// Transition to a new agent state and emit state transition event
    async fn transition_to_state(&mut self, new_state: AgentState) -> Result<(), MonitorError> {
        if self.current_state == new_state {
            return Ok(()); // No transition needed
        }

        let old_state = self.current_state.clone();
        self.current_state = new_state.clone();

        // Get current snapshot for the transition
        let snapshot = self.ht_client.take_snapshot().await?;

        let transition = StateTransition {
            from: old_state,
            to: new_state.clone(),
            timestamp: Instant::now(),
            snapshot,
        };

        info!(
            "Agent state transition: {:?} -> {:?}",
            transition.from, transition.to
        );

        // Send state transition event if we have a sender
        if let Some(tx) = &self.state_tx {
            if let Err(e) = tx.send(transition) {
                warn!("Failed to send state transition event: {}", e);
            }
        }

        Ok(())
    }

    /// Get monitoring statistics
    pub fn get_statistics(&self) -> MonitorStatistics {
        MonitorStatistics {
            current_state: self.current_state.clone(),
            snapshots_in_history: self.previous_snapshots.len(),
            time_since_last_output_change: self.last_output_change.elapsed(),
            is_running: self.running,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorStatistics {
    pub current_state: AgentState,
    pub snapshots_in_history: usize,
    pub time_since_last_output_change: Duration,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ht_process::{HtProcess, HtProcessConfig};
    use std::time::Duration;

    fn create_test_snapshot(content: &str) -> TerminalSnapshot {
        TerminalSnapshot {
            content: content.to_string(),
            width: 80,
            height: 24,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    #[tokio::test]
    async fn test_idle_state_detection() {
        let config = MonitorConfig::default();
        let prompt_patterns: Vec<Regex> = config
            .prompt_patterns
            .iter()
            .map(|p| Regex::new(p).unwrap())
            .collect();

        let test_config = HtProcessConfig::default();
        let monitor = TerminalOutputMonitor {
            ht_client: Arc::new(HtClient::new(HtProcess::new(test_config))), // Mock client
            config,
            event_receiver: None,
            state_tx: None,
            current_state: AgentState::Active,
            previous_snapshots: VecDeque::new(),
            prompt_patterns,
            last_output_change: Instant::now(),
            running: false,
        };

        // Test bash prompt detection
        let bash_snapshot = create_test_snapshot("user@hostname:/path$ ");
        assert!(monitor.is_idle_state(&bash_snapshot));

        // Test root prompt detection
        let root_snapshot = create_test_snapshot("root# ");
        assert!(monitor.is_idle_state(&root_snapshot));

        // Test non-prompt content
        let active_snapshot =
            create_test_snapshot("Running command...\nOutput line 1\nOutput line 2");
        assert!(!monitor.is_idle_state(&active_snapshot));
    }

    #[tokio::test]
    async fn test_output_change_detection() {
        let config = MonitorConfig::default();
        let test_config = HtProcessConfig::default();
        let mut monitor = TerminalOutputMonitor {
            ht_client: Arc::new(HtClient::new(HtProcess::new(test_config))), // Mock client
            config,
            event_receiver: None,
            state_tx: None,
            current_state: AgentState::Active,
            previous_snapshots: VecDeque::new(),
            prompt_patterns: vec![],
            last_output_change: Instant::now(),
            running: false,
        };

        // No previous snapshot - should detect change
        let snapshot1 = create_test_snapshot("Content 1");
        assert!(monitor.has_output_changed(&snapshot1));

        monitor.add_snapshot_to_history(snapshot1);

        // Same content - no change
        let snapshot2 = create_test_snapshot("Content 1");
        assert!(!monitor.has_output_changed(&snapshot2));

        // Different content - change detected
        let snapshot3 = create_test_snapshot("Content 2");
        assert!(monitor.has_output_changed(&snapshot3));
    }

    #[test]
    fn test_monitor_config_defaults() {
        let config = MonitorConfig::default();

        assert_eq!(config.change_detection_interval, Duration::from_millis(500));
        assert_eq!(config.wait_timeout, Duration::from_secs(2));
        assert_eq!(config.stuck_timeout, Duration::from_secs(30));
        assert_eq!(config.snapshot_history_size, 10);
        assert!(!config.prompt_patterns.is_empty());
    }

    #[test]
    fn test_agent_state_serialization() {
        let idle_state = AgentState::Idle;
        let serialized = serde_json::to_string(&idle_state).unwrap();
        let deserialized: AgentState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(idle_state, deserialized);
    }
}
