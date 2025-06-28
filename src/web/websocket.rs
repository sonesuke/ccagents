use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::agent::Agent;

pub async fn handle_websocket(socket: WebSocket, agent: Arc<Agent>) {
    info!("WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Send initial terminal state
    if let Ok(output) = agent.get_terminal_output().await {
        if let Err(e) = sender.send(Message::Text(output)).await {
            error!("Failed to send initial terminal output: {}", e);
            return;
        }
    }

    // Spawn task to handle incoming WebSocket messages (keyboard input)
    let agent_input = agent.clone();
    let input_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!(
                        "Received WebSocket input: {:?} (length: {})",
                        text,
                        text.len()
                    );
                    // Print each character for detailed debugging
                    for (i, ch) in text.chars().enumerate() {
                        debug!("  char[{}]: {:?} (code: {})", i, ch, ch as u32);
                    }
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

    // Spawn task to handle outgoing messages (terminal output)
    let agent_output = agent.clone();
    let output_task = tokio::spawn(async move {
        // Poll for terminal output updates
        let mut last_output = String::new();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            match agent_output.get_terminal_output().await {
                Ok(current_output) => {
                    if current_output != last_output {
                        if tx.send(current_output.clone()).await.is_err() {
                            debug!("Output channel closed");
                            break;
                        }
                        last_output = current_output;
                    }
                }
                Err(e) => {
                    error!("Failed to get terminal output: {}", e);
                }
            }
        }
    });

    // Forward output to WebSocket
    let forward_task = tokio::spawn(async move {
        while let Some(output) = rx.recv().await {
            if let Err(e) = sender.send(Message::Text(output)).await {
                error!("Failed to send terminal output to WebSocket: {}", e);
                break;
            }
        }
    });

    // Wait for any task to complete (connection closed or error)
    tokio::select! {
        _ = input_task => {
            debug!("Input task completed");
        }
        _ = output_task => {
            debug!("Output task completed");
        }
        _ = forward_task => {
            debug!("Forward task completed");
        }
    }

    info!("WebSocket connection closed");
}
