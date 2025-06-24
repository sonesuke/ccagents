use super::pty_session::{HtCommand, HtEvent, HtEventData, PtySession};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
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
pub enum HtMessage {
    #[serde(rename = "input")]
    Input { payload: String },
    #[serde(rename = "takeSnapshot")]
    TakeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HtResponse {
    View {
        view: Option<String>,
        status: String,
    },
    Snapshot {
        #[serde(rename = "type")]
        response_type: String,
        data: SnapshotData,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub seq: String,
    pub cols: u32,
    pub rows: u32,
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
    event_rx: Arc<Mutex<Option<broadcast::Receiver<HtEvent>>>>,
    response_tx: Arc<Mutex<Option<mpsc::UnboundedSender<HtResponse>>>>,
    response_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<HtResponse>>>>,
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
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            let command = HtCommand::Input { payload: input };
            session
                .handle_command(command)
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))?;
            Ok(())
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }

    pub async fn get_view(&self) -> Result<String, PtyProcessError> {
        let session_lock = self.session.lock().await;

        if let Some(session) = session_lock.as_ref() {
            session
                .handle_command(HtCommand::TakeSnapshot)
                .await
                .map_err(|e| PtyProcessError::CommunicationError(e.to_string()))?;

            drop(session_lock);
            let mut response_rx = self.response_rx.lock().await;

            if let Some(rx) = response_rx.as_mut() {
                match rx.recv().await {
                    Some(HtResponse::Snapshot { data, .. }) => Ok(data.seq),
                    Some(HtResponse::View { view, .. }) => view.ok_or_else(|| {
                        PtyProcessError::CommunicationError("No view data in response".to_string())
                    }),
                    None => Err(PtyProcessError::CommunicationError(
                        "No response received".to_string(),
                    )),
                }
            } else {
                Err(PtyProcessError::NotRunning)
            }
        } else {
            Err(PtyProcessError::NotRunning)
        }
    }
}

async fn event_processor(
    _session: Arc<PtySession>,
    event_rx: Arc<Mutex<Option<broadcast::Receiver<HtEvent>>>>,
    response_tx: mpsc::UnboundedSender<HtResponse>,
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
        if event.event_type.as_str() == "snapshot" {
            if let HtEventData::Snapshot {
                seq, cols, rows, ..
            } = event.data
            {
                let response = HtResponse::Snapshot {
                    response_type: "snapshot".to_string(),
                    data: SnapshotData {
                        seq,
                        cols: cols as u32,
                        rows: rows as u32,
                    },
                };

                if response_tx.send(response).is_err() {
                    break;
                }
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
