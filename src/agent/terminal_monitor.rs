use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSnapshot {
    pub content: String,
    pub cursor_position: Option<(u32, u32)>,
    pub width: u32,
    pub height: u32,
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
    #[allow(dead_code)]
    pub from: AgentState,
    #[allow(dead_code)]
    pub to: AgentState,
    #[allow(dead_code)]
    pub timestamp: Instant,
    #[allow(dead_code)]
    pub snapshot: TerminalSnapshot,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MonitorConfig {
    /// Interval between terminal snapshots for change detection
    #[allow(dead_code)]
    pub change_detection_interval: Duration,
    /// How long to wait without output changes before transitioning to Wait state
    #[allow(dead_code)]
    pub output_stable_duration: Duration,
    /// Regex patterns to detect shell prompts (indicates Idle state)
    #[allow(dead_code)]
    pub prompt_patterns: Vec<String>,
    /// Maximum number of snapshots to keep for comparison
    #[allow(dead_code)]
    pub max_snapshot_history: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            change_detection_interval: Duration::from_millis(500),
            output_stable_duration: Duration::from_secs(3),
            prompt_patterns: vec![
                r"\$\s*$".to_string(), // Bash prompt
                r">\s*$".to_string(),  // Windows prompt
                r"#\s*$".to_string(),  // Root prompt
                r"%\s*$".to_string(),  // Zsh default
                r"❯\s*$".to_string(),  // Popular custom prompt
                r"➜\s*$".to_string(),  // Another common prompt
            ],
            max_snapshot_history: 10,
        }
    }
}

#[allow(dead_code)]
pub struct TerminalOutputMonitor {
    agent_id: String,
    config: MonitorConfig,
    state_tx: Option<mpsc::UnboundedSender<StateTransition>>,
    current_state: AgentState,
    previous_snapshots: VecDeque<TerminalSnapshot>,
    prompt_patterns: Vec<Regex>,
    last_output_change: Instant,
    running: bool,
}

impl TerminalOutputMonitor {
    /// Create a new TerminalOutputMonitor
    #[allow(dead_code)]
    pub fn new(agent_id: String) -> Self {
        Self::with_config(agent_id, MonitorConfig::default())
    }

    /// Create a new TerminalOutputMonitor with custom configuration
    #[allow(dead_code)]
    pub fn with_config(agent_id: String, config: MonitorConfig) -> Self {
        // Compile regex patterns
        let prompt_patterns = config
            .prompt_patterns
            .iter()
            .filter_map(|pattern| match Regex::new(pattern) {
                Ok(regex) => Some(regex),
                Err(e) => {
                    warn!("Failed to compile prompt pattern '{}': {}", pattern, e);
                    None
                }
            })
            .collect();

        Self {
            agent_id,
            config,
            state_tx: None,
            current_state: AgentState::Idle,
            previous_snapshots: VecDeque::new(),
            prompt_patterns,
            last_output_change: Instant::now(),
            running: false,
        }
    }

    /// Start monitoring terminal output and return a receiver for state transitions
    #[allow(dead_code)]
    pub fn start_monitoring(&mut self) -> mpsc::UnboundedReceiver<StateTransition> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.state_tx = Some(tx);
        self.running = true;
        self.last_output_change = Instant::now();

        info!(
            "Terminal output monitoring started for agent {}",
            self.agent_id
        );
        rx
    }

    /// Stop monitoring
    #[allow(dead_code)]
    pub fn stop_monitoring(&mut self) {
        self.running = false;
        self.state_tx = None;
        info!(
            "Terminal output monitoring stopped for agent {}",
            self.agent_id
        );
    }

    /// Process a new terminal snapshot and detect state changes
    #[allow(dead_code)]
    pub async fn process_snapshot(&mut self, snapshot: TerminalSnapshot) -> Result<()> {
        if !self.running {
            return Err(anyhow::anyhow!("Monitor not running"));
        }

        let new_state = self.determine_state(&snapshot);

        // Check if state has changed
        if new_state != self.current_state {
            let transition = StateTransition {
                from: self.current_state.clone(),
                to: new_state.clone(),
                timestamp: Instant::now(),
                snapshot: snapshot.clone(),
            };

            info!(
                "Agent {} state transition: {:?} -> {:?}",
                self.agent_id, self.current_state, new_state
            );

            // Send state transition to listener
            if let Some(tx) = &self.state_tx {
                if let Err(e) = tx.send(transition) {
                    error!("Failed to send state transition: {}", e);
                }
            }

            self.current_state = new_state;
        }

        // Update snapshot history
        self.previous_snapshots.push_back(snapshot);
        if self.previous_snapshots.len() > self.config.max_snapshot_history {
            self.previous_snapshots.pop_front();
        }

        Ok(())
    }

    /// Determine the current state based on terminal snapshot
    fn determine_state(&mut self, snapshot: &TerminalSnapshot) -> AgentState {
        // Check if we're in idle state (shell prompt detected)
        if self.is_idle_state(snapshot) {
            return AgentState::Idle;
        }

        // Check if output has changed recently
        if self.has_output_changed(snapshot) {
            self.last_output_change = Instant::now();
            return AgentState::Active;
        }

        // If output hasn't changed for the configured duration, we're in Wait state
        if self.last_output_change.elapsed() >= self.config.output_stable_duration {
            return AgentState::Wait;
        }

        // Output is stable but not long enough to be in Wait state yet
        AgentState::Active
    }

    /// Check if the terminal is showing a shell prompt
    fn is_idle_state(&self, snapshot: &TerminalSnapshot) -> bool {
        // Get the last non-empty line
        let last_line = snapshot
            .content
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty());

        if let Some(line) = last_line {
            // Check if any prompt pattern matches
            for pattern in &self.prompt_patterns {
                if pattern.is_match(line) {
                    debug!("Prompt pattern matched: {}", line);
                    return true;
                }
            }
        }

        false
    }

    /// Check if terminal output has changed compared to previous snapshots
    fn has_output_changed(&self, snapshot: &TerminalSnapshot) -> bool {
        if self.previous_snapshots.is_empty() {
            return true;
        }

        // Compare with the most recent snapshot
        if let Some(last_snapshot) = self.previous_snapshots.back() {
            if last_snapshot.content != snapshot.content {
                return true;
            }

            // Also check cursor position changes
            if last_snapshot.cursor_position != snapshot.cursor_position {
                return true;
            }
        }

        false
    }

    /// Get the current state
    #[allow(dead_code)]
    pub fn current_state(&self) -> &AgentState {
        &self.current_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = TerminalOutputMonitor::new("test-agent".to_string());
        assert_eq!(monitor.current_state(), &AgentState::Idle);
        assert!(!monitor.running);
    }

    #[test]
    fn test_prompt_detection() {
        let config = MonitorConfig::default();
        let monitor = TerminalOutputMonitor::with_config("test-agent".to_string(), config);

        let snapshot = TerminalSnapshot {
            content: "user@host:/path$ ".to_string(),
            cursor_position: Some((17, 0)),
            width: 80,
            height: 24,
        };

        assert!(monitor.is_idle_state(&snapshot));
    }

    #[test]
    fn test_state_monitoring() {
        let mut monitor = TerminalOutputMonitor::new("test-agent".to_string());
        let _rx = monitor.start_monitoring();
        assert!(monitor.running);

        monitor.stop_monitoring();
        assert!(!monitor.running);
    }
}
