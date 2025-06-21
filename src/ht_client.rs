use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::agent::ht_process::{HtProcess, HtProcessError};

#[derive(Debug, Error)]
pub enum HtClientError {
    #[error("HT process error: {0}")]
    ProcessError(#[from] HtProcessError),
    #[error("Communication error: {0}")]
    CommunicationError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Command timeout")]
    Timeout,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HtCommand {
    #[serde(rename = "sendKeys")]
    SendKeys { keys: String },
    #[serde(rename = "takeSnapshot")]
    TakeSnapshot,
    #[serde(rename = "resize")]
    Resize { width: u32, height: u32 },
    #[serde(rename = "subscribe")]
    Subscribe { events: Vec<String> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { events: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HtEvent {
    #[serde(rename = "terminalOutput")]
    TerminalOutput { data: String },
    #[serde(rename = "terminalResize")]
    TerminalResize { width: u32, height: u32 },
    #[serde(rename = "processExit")]
    ProcessExit { code: i32 },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtCommandResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSnapshot {
    pub content: String,
    pub width: u32,
    pub height: u32,
    pub cursor_x: u32,
    pub cursor_y: u32,
}

pub struct HtClient {
    ht_process: Arc<HtProcess>,
    event_subscribers: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<HtEvent>>>>>,
    #[allow(dead_code)]
    command_id_counter: Arc<Mutex<u64>>,
}

impl HtClient {
    pub fn new(ht_process: HtProcess) -> Self {
        Self {
            ht_process: Arc::new(ht_process),
            event_subscribers: Arc::new(RwLock::new(HashMap::new())),
            command_id_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn start(&self) -> Result<(), HtClientError> {
        info!("Starting HT client");
        self.ht_process.start().await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), HtClientError> {
        info!("Stopping HT client");
        self.ht_process.stop().await?;
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        self.ht_process.is_running().await
    }

    pub async fn send_keys(&self, keys: &str) -> Result<(), HtClientError> {
        debug!("Sending keys: {}", keys);

        let command = HtCommand::SendKeys {
            keys: keys.to_string(),
        };

        self.send_command(command).await?;
        Ok(())
    }

    pub async fn take_snapshot(&self) -> Result<TerminalSnapshot, HtClientError> {
        debug!("Taking terminal snapshot");

        let command = HtCommand::TakeSnapshot;
        let response = self.send_command(command).await?;

        if let Some(data) = response.data {
            let snapshot: TerminalSnapshot = serde_json::from_value(data)?;
            Ok(snapshot)
        } else {
            Err(HtClientError::InvalidResponse(
                "No snapshot data in response".to_string(),
            ))
        }
    }

    pub async fn resize(&self, width: u32, height: u32) -> Result<(), HtClientError> {
        debug!("Resizing terminal to {}x{}", width, height);

        let command = HtCommand::Resize { width, height };
        self.send_command(command).await?;
        Ok(())
    }

    pub async fn subscribe_to_events(
        &self,
        events: Vec<String>,
    ) -> Result<mpsc::UnboundedReceiver<HtEvent>, HtClientError> {
        debug!("Subscribing to events: {:?}", events);

        let (tx, rx) = mpsc::unbounded_channel();

        // Add subscriber to internal registry
        {
            let mut subscribers = self.event_subscribers.write().await;
            for event in &events {
                subscribers
                    .entry(event.clone())
                    .or_insert_with(Vec::new)
                    .push(tx.clone());
            }
        }

        // Send subscription command to HT process
        let command = HtCommand::Subscribe { events };
        self.send_command(command).await?;

        Ok(rx)
    }

    pub async fn unsubscribe_from_events(&self, events: Vec<String>) -> Result<(), HtClientError> {
        debug!("Unsubscribing from events: {:?}", events);

        // Remove subscribers from internal registry
        {
            let mut subscribers = self.event_subscribers.write().await;
            for event in &events {
                subscribers.remove(event);
            }
        }

        // Send unsubscription command to HT process
        let command = HtCommand::Unsubscribe { events };
        self.send_command(command).await?;

        Ok(())
    }

    async fn send_command(&self, command: HtCommand) -> Result<HtCommandResponse, HtClientError> {
        let command_json = serde_json::to_string(&command)?;
        debug!("Sending command: {}", command_json);

        // For now, we'll use the existing HT process methods
        // In a real implementation, this would be more sophisticated
        match command {
            HtCommand::SendKeys { keys } => {
                self.ht_process.send_input(keys).await?;
                Ok(HtCommandResponse {
                    success: true,
                    data: None,
                    error: None,
                })
            }
            HtCommand::TakeSnapshot => {
                let view = self.ht_process.get_view().await?;
                let snapshot = TerminalSnapshot {
                    content: view,
                    width: 80, // Default values - would be dynamically determined
                    height: 24,
                    cursor_x: 0,
                    cursor_y: 0,
                };
                Ok(HtCommandResponse {
                    success: true,
                    data: Some(serde_json::to_value(snapshot)?),
                    error: None,
                })
            }
            HtCommand::Resize {
                width: _,
                height: _,
            } => {
                // Resize functionality would be implemented here
                // For now, just return success
                Ok(HtCommandResponse {
                    success: true,
                    data: None,
                    error: None,
                })
            }
            HtCommand::Subscribe { events: _ } => {
                // Event subscription would be implemented here
                Ok(HtCommandResponse {
                    success: true,
                    data: None,
                    error: None,
                })
            }
            HtCommand::Unsubscribe { events: _ } => {
                // Event unsubscription would be implemented here
                Ok(HtCommandResponse {
                    success: true,
                    data: None,
                    error: None,
                })
            }
        }
    }

    #[allow(dead_code)]
    async fn handle_event(&self, event: HtEvent) {
        debug!("Handling event: {:?}", event);

        let event_type = match &event {
            HtEvent::TerminalOutput { .. } => "terminalOutput",
            HtEvent::TerminalResize { .. } => "terminalResize",
            HtEvent::ProcessExit { .. } => "processExit",
            HtEvent::Error { .. } => "error",
        };

        let subscribers = self.event_subscribers.read().await;
        if let Some(subs) = subscribers.get(event_type) {
            for sender in subs {
                if let Err(e) = sender.send(event.clone()) {
                    warn!("Failed to send event to subscriber: {}", e);
                }
            }
        }
    }

    #[allow(dead_code)]
    async fn get_next_command_id(&self) -> u64 {
        let mut counter = self.command_id_counter.lock().await;
        *counter += 1;
        *counter
    }
}

impl Drop for HtClient {
    fn drop(&mut self) {
        // Cleanup resources if needed
        debug!("HtClient dropped");
    }
}
