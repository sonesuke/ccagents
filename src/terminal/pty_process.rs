use super::pty_session::{PtyCommand, PtyEvent, PtyEventData, PtySession};
use crate::config::Config;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, broadcast, mpsc};
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

impl PtyProcessConfig {
    /// Create PtyProcessConfig from Config
    pub fn from_config(config: &Config) -> Self {
        let (cols, rows) = (config.web_ui.cols, config.web_ui.rows);
        Self {
            shell_command: Some(std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())),
            cols,
            rows,
        }
    }
}

pub struct PtyProcess {
    config: PtyProcessConfig,
    session: Arc<Mutex<Option<Arc<PtySession>>>>,
    event_rx: Arc<Mutex<Option<broadcast::Receiver<PtyEvent>>>>,
    response_tx: Arc<Mutex<Option<mpsc::UnboundedSender<PtyResponse>>>>,
    response_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<PtyResponse>>>>,
}

impl PtyProcess {
    pub fn new(config: PtyProcessConfig) -> Self {
        Self {
            config,
            session: Arc::new(Mutex::new(None)),
            event_rx: Arc::new(Mutex::new(None)),
            response_tx: Arc::new(Mutex::new(None)),
            response_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Create PtyProcess directly from Config
    pub fn from_config(config: &Config) -> Self {
        let pty_config = PtyProcessConfig::from_config(config);
        Self::new(pty_config)
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

        *session_lock = Some(session.clone());
        *self.event_rx.lock().await = Some(event_rx);
        *self.response_tx.lock().await = Some(response_tx.clone());
        *self.response_rx.lock().await = Some(response_rx);

        tokio::spawn(event_processor(
            session.clone(),
            self.event_rx.clone(),
            response_tx,
        ));

        info!("PTY process started successfully");
        Ok(())
    }

    pub async fn send_input(&self, input: String) -> Result<(), PtyProcessError> {
        info!("🔍 send_input called with: {:?}", input);

        info!("🔐 Attempting to acquire session lock...");
        let session_lock = self.session.lock().await;
        info!("✅ Session lock acquired");

        if let Some(session) = session_lock.as_ref() {
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

    /// Get direct access to PTY raw bytes receiver for WebSocket streaming
    pub async fn get_pty_bytes_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<bytes::Bytes>, PtyProcessError> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session
                .get_pty_bytes_receiver()
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }

    /// Get direct access to PTY string receiver for rule matching
    pub async fn get_pty_string_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<String>, PtyProcessError> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session
                .get_pty_string_receiver()
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }

    /// Get current screen contents for WebSocket initial state
    pub async fn get_screen_contents(&self) -> Result<String, PtyProcessError> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session
                .get_screen_contents()
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }

    /// Get the PID of the shell process
    pub async fn get_shell_pid(&self) -> Result<Option<u32>, PtyProcessError> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session
                .get_shell_pid()
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }

    /// Get child processes of the shell process
    pub async fn get_child_processes(&self) -> Result<Vec<u32>, PtyProcessError> {
        if let Ok(Some(shell_pid)) = self.get_shell_pid().await {
            let output = Command::new("pgrep")
                .arg("-P")
                .arg(shell_pid.to_string())
                .output()
                .map_err(PtyProcessError::IoError)?;

            if output.status.success() {
                let child_pids = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| line.trim().parse::<u32>().ok())
                    .collect();
                Ok(child_pids)
            } else {
                Ok(Vec::new())
            }
        } else {
            Ok(Vec::new())
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
