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
        .send(Message::Text(header.to_string().into()))
        .await
        .is_err()
    {
        error!("Failed to send asciinema header");
        return;
    }

    info!("✅ Asciinema header sent successfully");

    // Send initial state from vt100::Parser screen contents
    match agent.process.get_screen_contents().await {
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
                    .send(Message::Text(initial_event.to_string().into()))
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

    // Full-screen redraw approach instead of incremental updates
    let agent_output = agent.clone();
    let session_start = std::time::Instant::now();

    let output_task = tokio::spawn(async move {
        info!("🔄 WebSocket full-screen output task started");

        // Get direct access to PTY raw bytes broadcast channel
        if let Ok(mut pty_bytes_rx) = agent_output.process.get_pty_bytes_receiver().await {
            info!("✅ Connected to PTY raw bytes broadcast channel");

            info!("🔄 WebSocket: Starting recv loop for full-screen updates");

            // Track last screen content to avoid redundant updates
            let mut last_screen_content = String::new();
            let mut last_update = std::time::Instant::now();
            const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100); // ~10fps
            const DEBOUNCE_TIME: std::time::Duration = std::time::Duration::from_millis(50); // Debounce rapid changes

            while let Ok(_bytes_data) = pty_bytes_rx.recv().await {
                // Wait for debounce time or update interval
                if last_update.elapsed() < UPDATE_INTERVAL {
                    // For rapid changes, wait a bit to accumulate
                    if last_update.elapsed() < DEBOUNCE_TIME {
                        tokio::time::sleep(DEBOUNCE_TIME - last_update.elapsed()).await;
                    } else {
                        continue;
                    }
                }

                // Get full screen contents from vt100 parser
                match agent_output.process.get_screen_contents().await {
                    Ok(screen_content) => {
                        // Only send if screen content has changed
                        if screen_content != last_screen_content {
                            // Calculate elapsed time from session start
                            let time = session_start.elapsed().as_secs_f64();

                            // Clear screen and redraw
                            let clear_screen = "\u{001b}[2J\u{001b}[H"; // Clear screen and move cursor to home
                            let full_update = format!("{}{}", clear_screen, screen_content);

                            // Create asciinema event with full screen content
                            let asciinema_event = json!([time, "o", full_update]);
                            let event_str = asciinema_event.to_string();

                            info!(
                                "📤 Sending full screen update: {} bytes at {:.3}s",
                                event_str.len(),
                                time
                            );

                            if sender.send(Message::Text(event_str.into())).await.is_err() {
                                info!("WebSocket sender closed, stopping output task");
                                break;
                            }

                            info!("✅ Full screen update sent successfully");
                            last_screen_content = screen_content;
                            last_update = std::time::Instant::now();
                        }
                    }
                    Err(e) => {
                        error!("❌ Failed to get screen contents: {}", e);
                    }
                }
            }
            info!("🔚 WebSocket: Full screen update loop ended");
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
