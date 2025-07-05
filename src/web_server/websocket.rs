use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{debug, error, info};

use crate::agent::Agent;

/// Terminal session with avt virtual terminal for proper terminal state management
pub struct TerminalSession {
    vt: avt::Vt,
    start_time: std::time::Instant,
}

impl TerminalSession {
    /// Create a new terminal session with specified dimensions
    pub fn new(cols: u16, rows: u16) -> Self {
        let vt = avt::Vt::new(rows as usize, cols as usize);
        Self {
            vt,
            start_time: std::time::Instant::now(),
        }
    }

    /// Feed PTY output string to the virtual terminal
    pub fn feed_output(&mut self, data: &str) {
        self.vt.feed_str(data);
    }

    /// Get elapsed time since session start
    pub fn elapsed_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

pub async fn handle_websocket(socket: WebSocket, agent: Arc<Agent>) {
    info!("WebSocket connection established for asciinema streaming");

    let (mut sender, mut receiver) = socket.split();

    // Send asciinema header first with dynamic terminal size
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Get actual terminal dimensions from agent config and create terminal session
    let (cols, rows) = agent.get_terminal_size();
    let mut terminal_session = TerminalSession::new(cols, rows);

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

    // Send accumulated terminal state to new client
    if let Ok(accumulated_output) = agent.get_accumulated_output().await {
        if !accumulated_output.is_empty() {
            let time = 0.0; // Initial state at time 0
            let initial_event = json!([time, "o", accumulated_output]);
            let event_str = initial_event.to_string();

            info!(
                "📤 Sending initial terminal state: {} bytes (accumulated output)",
                event_str.len()
            );
            debug!(
                "Initial state preview: {:?}",
                &accumulated_output[..std::cmp::min(200, accumulated_output.len())]
            );

            if sender.send(Message::Text(event_str)).await.is_err() {
                error!("Failed to send initial terminal state");
                return;
            }
            info!("✅ Initial terminal state sent successfully");
        } else {
            info!("⚠️ No accumulated output available for initial state");
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

    // Event-driven output handling using avt virtual terminal
    let agent_output = agent.clone();

    let output_task = tokio::spawn(async move {
        info!("🔄 WebSocket avt-based output task started");

        // Get direct access to PTY raw bytes broadcast channel
        if let Ok(mut pty_bytes_rx) = agent_output.get_pty_bytes_receiver().await {
            info!("✅ Connected to PTY raw bytes broadcast channel");

            info!("🔄 WebSocket: Starting recv loop for PTY raw bytes with avt processing and buffering");

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
                    // Convert buffered bytes to string and feed to virtual terminal
                    let string_data = String::from_utf8_lossy(&output_buffer).to_string();

                    info!(
                        "🔍 WebSocket: Processing {} buffered bytes through avt",
                        output_buffer.len()
                    );

                    // Feed the string data to avt virtual terminal
                    terminal_session.feed_output(&string_data);

                    // Get elapsed time from terminal session
                    let time = terminal_session.elapsed_time();

                    // Create asciinema event with the buffered string data
                    let asciinema_event = json!([time, "o", string_data]);
                    let event_str = asciinema_event.to_string();

                    info!(
                        "📤 Sending buffered avt-processed asciinema event: {} bytes at {:.3}s",
                        event_str.len(),
                        time
                    );

                    if sender.send(Message::Text(event_str)).await.is_err() {
                        info!("WebSocket sender closed, stopping output task");
                        break;
                    }
                    info!("✅ Buffered avt-processed asciinema event sent successfully");

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
