use super::pty_session::{PtyCommand, PtyEvent, PtyEventData, PtySession};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum PtyProcessError {
    #[error("PTY process failed to start: {0}")]
    StartupFailure(String),
    #[error("PTY process communication error: {0}")]
    CommunicationError(String),
    #[error("PTY process not running")]
    NotRunning,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PtyMessage {
    #[serde(rename = "input")]
    Input { payload: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PtyResponse {
    View {
        view: Option<String>,
        status: String,
    },
    Output {
        #[serde(rename = "type")]
        response_type: String,
        data: String,
    },
}

/// Command process output monitor
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub content: String,

    pub is_stdout: bool, // true for stdout, false for stderr
}

#[derive(Debug, Clone)]
pub struct PtyProcessConfig {
    pub shell_command: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

impl Default for PtyProcessConfig {
    fn default() -> Self {
        Self {
            shell_command: Some(std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())),
            cols: 80,
            rows: 24,
        }
    }
}

pub struct PtyProcess {
    config: PtyProcessConfig,
    session: Arc<Mutex<Option<Arc<PtySession>>>>,
    event_rx: Arc<Mutex<Option<broadcast::Receiver<PtyEvent>>>>,
    response_tx: Arc<Mutex<Option<mpsc::UnboundedSender<PtyResponse>>>>,
    response_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<PtyResponse>>>>,
    // Claude output monitoring
    command_output_tx: Arc<Mutex<Option<mpsc::UnboundedSender<CommandOutput>>>>,
    command_output_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<CommandOutput>>>>,
}

impl PtyProcess {
    pub fn new(config: PtyProcessConfig) -> Self {
        Self {
            config,
            session: Arc::new(Mutex::new(None)),
            event_rx: Arc::new(Mutex::new(None)),
            response_tx: Arc::new(Mutex::new(None)),
            response_rx: Arc::new(Mutex::new(None)),
            command_output_tx: Arc::new(Mutex::new(None)),
            command_output_rx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> Result<(), PtyProcessError> {
        let mut session_lock = self.session.lock().await;

        if session_lock.is_some() {
            warn!("PTY process is already running");
            return Ok(());
        }

        info!("Starting PTY process with config: {:?}", self.config);

        let shell = self.config.shell_command.as_deref().unwrap_or("bash");
        let session = Arc::new(
            PtySession::new(
                shell.to_string(),
                self.config.cols as usize,
                self.config.rows as usize,
            )
            .await
            .map_err(|e| PtyProcessError::StartupFailure(e.to_string()))?,
        );

        let event_rx = session.subscribe().await;
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        let (command_output_tx, command_output_rx) = mpsc::unbounded_channel();

        *session_lock = Some(session.clone());
        *self.event_rx.lock().await = Some(event_rx);
        *self.response_tx.lock().await = Some(response_tx.clone());
        *self.response_rx.lock().await = Some(response_rx);
        *self.command_output_tx.lock().await = Some(command_output_tx.clone());
        *self.command_output_rx.lock().await = Some(command_output_rx);

        tokio::spawn(event_processor(
            session.clone(),
            self.event_rx.clone(),
            response_tx,
        ));

        info!("PTY process started successfully");
        Ok(())
    }

    pub async fn send_input(&self, input: String) -> Result<(), PtyProcessError> {
        // DETAILED DEBUG LOGGING
        tracing::debug!("=== SEND_INPUT CALLED ===");
        tracing::debug!("Raw input: {:?}", input);
        tracing::debug!("Trimmed input: {:?}", input.trim());
        tracing::debug!("Input length: {}", input.len());
        tracing::debug!(
            "Starts with 'claude ': {}",
            input.trim().starts_with("claude ")
        );
        tracing::debug!("---");

        info!("🔍 send_input called with: {:?}", input);

        info!("🔐 Attempting to acquire session lock...");
        let session_lock = self.session.lock().await;
        info!("✅ Session lock acquired");

        if let Some(session) = session_lock.as_ref() {
            // Check if this is a claude command and start monitoring
            let should_monitor = input.trim().starts_with("claude ");

            if should_monitor {
                info!("🎯 Detected claude command, starting output monitoring");
                tracing::debug!("🎯 Detected claude command: {}", input.trim());
                self.start_command_monitoring(&input).await?;
            } else {
                info!("❌ Not a claude command: '{}'", input.trim());
            }

            let command = PtyCommand::Input { payload: input };
            info!("📨 About to call session.handle_command");
            session
                .handle_command(command)
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))?;
            info!("✅ session.handle_command completed");
            Ok(())
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }

    /// Start monitoring command process output
    async fn start_command_monitoring(&self, command: &str) -> Result<(), PtyProcessError> {
        tracing::debug!("=== START_COMMAND_MONITORING ===");
        tracing::debug!("Command: {:?}", command);

        let command_output_tx = self.command_output_tx.lock().await;

        if let Some(tx) = command_output_tx.as_ref() {
            tracing::debug!("✅ Command monitoring channel available, spawning monitor task");

            tracing::debug!("✅ Channel available, spawning monitor task");

            let tx_clone = tx.clone();
            let command_clone = command.to_string();

            // Spawn a background task to monitor command process
            tokio::spawn(async move {
                tracing::debug!("🚀 Command monitor task started");
                if let Err(e) = Self::monitor_command_process(command_clone, tx_clone).await {
                    error!("Command monitoring failed: {}", e);
                    println!("❌ Command monitoring failed: {}", e);
                }
            });
        } else {
            println!("❌ Command monitoring channel not available");

            tracing::debug!("❌ Channel NOT available");
        }

        Ok(())
    }

    /// Parse shell command with proper quote handling
    fn parse_shell_command(command: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let chars = command.chars();

        for ch in chars {
            match ch {
                '\'' | '"' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                '\'' | '"' if in_quotes && ch == quote_char => {
                    in_quotes = false;
                    quote_char = ' ';
                }
                ' ' | '\t' if !in_quotes => {
                    if !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                    }
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        if !current_arg.is_empty() {
            args.push(current_arg);
        }

        args
    }

    /// Monitor command process by executing it separately and capturing stdout/stderr
    async fn monitor_command_process(
        command: String,
        output_tx: mpsc::UnboundedSender<CommandOutput>,
    ) -> Result<(), PtyProcessError> {
        tracing::debug!("=== MONITOR_COMMAND_PROCESS ===");
        tracing::debug!("Command: {:?}", command);

        // Parse command to extract arguments with proper quote handling
        let args = Self::parse_shell_command(&command);

        tracing::debug!("Parsed args: {:?}", args);

        if args.is_empty() {
            println!("❌ Invalid command for monitoring: {:?}", command);
            tracing::debug!("❌ Invalid command: args={:?}", args);
            return Ok(());
        }

        let command_name = &args[0];
        let command_args: Vec<String> = args[1..].to_vec();

        info!(
            "Starting process monitoring: {} with args: {:?}",
            command_name, command_args
        );
        tracing::debug!(
            "🔍 Starting process monitoring: {} with args: {:?}",
            command_name,
            command_args
        );

        tracing::debug!("Command: {} args: {:?}", command_name, command_args);
        tracing::debug!("About to spawn process");

        // Start command process with separate stdout/stderr capture
        let spawn_result = Command::new(command_name)
            .args(&command_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match spawn_result {
            Ok(child) => {
                tracing::debug!("✅ Process spawned successfully: {}", command_name);
                child
            }
            Err(e) => {
                println!("❌ Failed to spawn process {}: {}", command_name, e);
                tracing::debug!("❌ Failed to spawn {}: {}", command_name, e);
                return Err(PtyProcessError::IoError(e));
            }
        };

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let tx_stdout = output_tx.clone();
            let command_name_clone = command_name.to_string();
            tokio::spawn(async move {
                tracing::debug!("📡 Starting stdout monitoring for {}", command_name_clone);
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!("📤 {} stdout: {:?}", command_name_clone, line);
                    if !line.trim().is_empty() {
                        let output = CommandOutput {
                            content: line.clone(),
                            is_stdout: true,
                        };
                        if let Err(e) = tx_stdout.send(output) {
                            println!("❌ Failed to send stdout: {}", e);
                            break;
                        } else {
                            tracing::debug!("✅ Sent stdout to channel: {:?}", line);
                        }
                    }
                }
                tracing::debug!("📡 Stdout monitoring ended for {}", command_name_clone);
            });
        } else {
            println!("❌ No stdout pipe available");
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let tx_stderr = output_tx.clone();
            let command_name_clone = command_name.to_string();
            tokio::spawn(async move {
                tracing::debug!("📡 Starting stderr monitoring for {}", command_name_clone);
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!("📤 {} stderr: {:?}", command_name_clone, line);
                    if !line.trim().is_empty() {
                        let output = CommandOutput {
                            content: line.clone(),
                            is_stdout: false,
                        };
                        if let Err(e) = tx_stderr.send(output) {
                            println!("❌ Failed to send stderr: {}", e);
                            break;
                        } else {
                            tracing::debug!("✅ Sent stderr to channel: {:?}", line);
                        }
                    }
                }
                tracing::debug!("📡 Stderr monitoring ended for {}", command_name_clone);
            });
        } else {
            println!("❌ No stderr pipe available");
        }

        // Wait for process to complete
        let exit_status = child.wait().await;
        info!("Process monitoring completed: {}", command_name);
        tracing::debug!(
            "🏁 Process {} completed with status: {:?}",
            command_name,
            exit_status
        );

        Ok(())
    }

    /// Get command output (non-blocking)
    pub async fn get_command_output(&self) -> Option<CommandOutput> {
        let mut rx_lock = self.command_output_rx.lock().await;
        if let Some(rx) = rx_lock.as_mut() {
            rx.try_recv().ok()
        } else {
            None
        }
    }

    /// Get accumulated terminal output for initial WebSocket state
    pub async fn get_accumulated_output(&self) -> Vec<u8> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session.get_accumulated_output().await
        } else {
            Vec::new()
        }
    }

    /// Get direct access to PTY output broadcast receiver for WebSocket streaming
    pub async fn get_pty_output_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<String>, PtyProcessError> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session
                .get_pty_output_receiver()
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }
}

async fn event_processor(
    session: Arc<PtySession>,
    event_rx: Arc<Mutex<Option<broadcast::Receiver<PtyEvent>>>>,
    response_tx: mpsc::UnboundedSender<PtyResponse>,
) {
    let mut rx = {
        let guard = event_rx.lock().await;
        if let Some(rx) = guard.as_ref() {
            rx.resubscribe()
        } else {
            return;
        }
    };

    while let Ok(event) = rx.recv().await {
        match event.event_type.as_str() {
            "output" => {
                if let PtyEventData::Output { data } = event.data {
                    info!(
                        "🎉 Processing Output event: {} bytes: {:?}",
                        data.len(),
                        data
                    );

                    // Send the output data to the terminal's output channel
                    if let Err(e) = session.send_output_data(&data).await {
                        error!("Failed to send output to terminal: {}", e);
                    } else {
                        info!("✅ Output data sent to terminal successfully");
                    }

                    let response = PtyResponse::Output {
                        response_type: "output".to_string(),
                        data,
                    };

                    if response_tx.send(response).is_err() {
                        error!("❌ Failed to send output response");
                        break;
                    } else {
                        info!("✅ Output response sent successfully");
                    }
                }
            }
            _ => {
                // Ignore other event types for now
            }
        }
    }
}

impl Drop for PtyProcess {
    fn drop(&mut self) {
        if let Ok(mut session) = self.session.try_lock() {
            session.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shell_command() {
        let test_cases = vec![
            (
                "claude 'say hello, in Japanese'",
                vec!["claude", "say hello, in Japanese"],
            ),
            ("claude \"hello world\"", vec!["claude", "hello world"]),
            ("claude simple", vec!["claude", "simple"]),
            ("claude arg1 arg2", vec!["claude", "arg1", "arg2"]),
            (
                "claude 'complex arg' another",
                vec!["claude", "complex arg", "another"],
            ),
        ];

        for (input, expected) in test_cases {
            let result = PtyProcess::parse_shell_command(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
}
