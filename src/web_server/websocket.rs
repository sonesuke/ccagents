use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, error, info};

use crate::agent::Agent;

pub async fn handle_websocket(socket: WebSocket, agent: Arc<Agent>) {
    info!("WebSocket connection established for direct ANSI streaming");

    let (mut sender, mut receiver) = socket.split();

    // Spawn task to handle incoming WebSocket messages (keyboard input)
    let agent_input = agent.clone();
    let input_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received input: {:?}", text);
                    if let Err(e) = agent_input.send_input(&text).await {
                        error!("Failed to send input to agent: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by client");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Spawn task to stream raw ANSI output directly
    let agent_output = agent.clone();
    let output_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;

            match agent_output.get_raw_ansi_output().await {
                Ok(Some(output)) => {
                    if !output.is_empty() {
                        info!("📤 Sending WebSocket data: {} bytes", output.len());
                        if sender.send(Message::Text(output)).await.is_err() {
                            debug!("WebSocket sender closed");
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // No new output, continue
                }
                Err(e) => {
                    error!("Failed to get raw ANSI output: {}", e);
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = input_task => {
            debug!("Input task completed");
        }
        _ = output_task => {
            debug!("Output task completed");
        }
    }

    info!("WebSocket connection closed");
}
