use std::sync::Arc;
use std::time::Instant;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{debug, error, info};

use crate::agent::Agent;

#[derive(Debug, Clone)]
pub enum TerminalEvent {
    Init {
        time: f64,
        cols: usize,
        rows: usize,
        screen_dump: String,
    },
    #[allow(dead_code)]
    Output { time: f64, data: String },
    #[allow(dead_code)]
    Resize { time: f64, cols: usize, rows: usize },
}

pub async fn handle_websocket(socket: WebSocket, agent: Arc<Agent>) {
    info!("WebSocket connection established");
    println!("🔌 WebSocket connection established for ALiS protocol");

    let (mut sender, mut receiver) = socket.split();
    let start_time = Instant::now();

    // Wait a bit for terminal to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send HTプロジェクト style initial event
    let (cols, rows) = agent.get_terminal_size();

    // Get initial screen dump
    let initial_content: String = agent.get_terminal_output().await.unwrap_or_default();

    let init_event = TerminalEvent::Init {
        time: 0.0,
        cols: cols as usize,
        rows: rows as usize,
        screen_dump: initial_content.clone(),
    };

    let init_message = match &init_event {
        TerminalEvent::Init {
            time,
            cols,
            rows,
            screen_dump,
        } => {
            json!({
                "type": "init",
                "time": time,
                "cols": cols,
                "rows": rows,
                "data": screen_dump
            })
        }
        _ => unreachable!(),
    };

    println!(
        "📋 Sending HT-style init event: cols={}, rows={}",
        cols, rows
    );

    if let Err(e) = sender.send(Message::Text(init_message.to_string())).await {
        error!("Failed to send init event: {}", e);
        return;
    }

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

    // Spawn task to stream terminal output in asciinema format
    let agent_output = agent.clone();
    let start_time_clone = start_time;
    let output_task = tokio::spawn(async move {
        let mut last_output = String::new();
        let _buffer = String::new();

        // Wait a bit for terminal to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            match agent_output.get_terminal_output().await {
                Ok(current_output) => {
                    if current_output != last_output && !current_output.trim().is_empty() {
                        // Find what's new since last update
                        let new_content = if last_output.is_empty() {
                            current_output.clone()
                        } else if current_output.len() > last_output.len()
                            && current_output.starts_with(&last_output)
                        {
                            // Only send the new part if it's an append
                            current_output[last_output.len()..].to_string()
                        } else if current_output != last_output {
                            // Different content, send full screen for now
                            format!("\x1b[2J\x1b[H{}", current_output)
                        } else {
                            String::new()
                        };

                        if !new_content.is_empty() {
                            // Create HT-style output event
                            let timestamp = start_time_clone.elapsed().as_secs_f64();
                            let output_event = json!({
                                "type": "output",
                                "time": timestamp,
                                "data": new_content
                            });

                            debug!("Sending incremental update: {} chars", new_content.len());
                            println!(
                                "📨 Sending HT-style output: timestamp={:.3}, content_len={}",
                                timestamp,
                                new_content.len()
                            );

                            if sender
                                .send(Message::Text(output_event.to_string()))
                                .await
                                .is_err()
                            {
                                debug!("WebSocket sender closed");
                                println!("❌ WebSocket sender closed");
                                break;
                            }
                        }

                        last_output = current_output;
                    }
                }
                Err(e) => {
                    error!("Failed to get terminal output: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
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
