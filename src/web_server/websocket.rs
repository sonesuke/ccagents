use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{debug, error, info};

use crate::agent::Agent;

pub async fn handle_websocket(socket: WebSocket, agent: Arc<Agent>) {
    info!("WebSocket connection established for asciinema streaming");

    let (mut sender, mut receiver) = socket.split();

    // Send asciinema header first with dynamic terminal size
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Get actual terminal dimensions from agent config
    let (cols, rows) = agent.get_terminal_size();

    let header = json!({
        "version": 2,
        "width": cols,
        "height": rows,
        "timestamp": start_time,
        "env": {
            "TERM": "xterm-256color",
            "SHELL": "/bin/bash"
        }
    });

    info!(
        "📐 Using terminal dimensions: {}x{} from config",
        cols, rows
    );

    if sender
        .send(Message::Text(header.to_string()))
        .await
        .is_err()
    {
        error!("Failed to send asciinema header");
        return;
    }

    info!("✅ Asciinema header sent successfully");

    // Send initial state from vt100::Parser screen contents
    match agent.get_screen_contents().await {
        Ok(initial_content) => {
            if !initial_content.trim().is_empty() {
                let initial_time = 0.0;
                let initial_event = json!([initial_time, "o", initial_content]);

                info!(
                    "📺 Sending initial terminal state from vt100: {} bytes at time {:.3}s",
                    initial_content.len(),
                    initial_time
                );
                info!(
                    "🔍 Initial content preview (first 200 chars): {:?}",
                    initial_content.chars().take(200).collect::<String>()
                );

                if sender
                    .send(Message::Text(initial_event.to_string()))
                    .await
                    .is_err()
                {
                    error!("Failed to send initial terminal state");
                    return;
                }

                info!("✅ Initial terminal state sent successfully");
            } else {
                info!("📺 No terminal content to send (empty screen)");
            }
        }
        Err(e) => {
            error!("❌ Failed to get screen contents: {}", e);
            info!("📺 Proceeding without initial state");
        }
    }

    // Spawn task to handle incoming WebSocket messages (not used in asciinema mode)
    let input_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
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

    // Event-driven output handling with direct PTY data streaming
    let agent_output = agent.clone();
    let session_start = std::time::Instant::now();

    let output_task = tokio::spawn(async move {
        info!("🔄 WebSocket direct PTY output task started");

        // Get direct access to PTY raw bytes broadcast channel
        if let Ok(mut pty_bytes_rx) = agent_output.get_pty_bytes_receiver().await {
            info!("✅ Connected to PTY raw bytes broadcast channel");

            info!("🔄 WebSocket: Starting recv loop for direct PTY raw bytes streaming");

            // Buffer for accumulating output (similar to ht project)
            let mut output_buffer = Vec::new();
            let mut last_send = std::time::Instant::now();
            const BUFFER_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(16); // ~60fps
            const MAX_BUFFER_SIZE: usize = 4096; // Reasonable buffer size

            while let Ok(bytes_data) = pty_bytes_rx.recv().await {
                // Accumulate bytes in buffer
                output_buffer.extend_from_slice(&bytes_data);

                info!(
                    "🔍 WebSocket: Buffering {} bytes (total buffered: {})",
                    bytes_data.len(),
                    output_buffer.len()
                );

                // Send buffer if timeout elapsed or buffer is large enough
                let should_send =
                    last_send.elapsed() >= BUFFER_TIMEOUT || output_buffer.len() >= MAX_BUFFER_SIZE;

                if should_send && !output_buffer.is_empty() {
                    // Convert buffered bytes to string for asciicast v2 format
                    let string_data = String::from_utf8_lossy(&output_buffer).to_string();

                    info!(
                        "🔍 WebSocket: Processing {} buffered bytes for direct streaming",
                        output_buffer.len()
                    );

                    // Calculate elapsed time from session start
                    let time = session_start.elapsed().as_secs_f64();

                    // Create asciinema event with the raw buffered data (asciicast v2 format)
                    let asciinema_event = json!([time, "o", string_data]);
                    let event_str = asciinema_event.to_string();

                    info!(
                        "📤 Sending direct PTY asciinema event: {} bytes at {:.3}s",
                        event_str.len(),
                        time
                    );

                    if sender.send(Message::Text(event_str)).await.is_err() {
                        info!("WebSocket sender closed, stopping output task");
                        break;
                    }
                    info!("✅ Direct PTY asciinema event sent successfully");

                    // Clear buffer and update send time
                    output_buffer.clear();
                    last_send = std::time::Instant::now();
                }
            }
            info!("🔚 WebSocket: PTY output recv loop ended");
        } else {
            error!("❌ Failed to get PTY output receiver from agent");
        }

        info!("🔚 WebSocket output task terminated");
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
