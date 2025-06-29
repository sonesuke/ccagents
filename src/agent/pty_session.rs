use super::pty_terminal::PtyTerminal;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PtyCommand {
    #[serde(rename = "input")]
    Input { payload: String },
    #[serde(rename = "sendKeys")]
    SendKeys { keys: Vec<String> },
    #[serde(rename = "resize")]
    Resize { cols: usize, rows: usize },
    #[serde(rename = "takeSnapshot")]
    TakeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub time: f64,
    #[serde(flatten)]
    pub data: PtyEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PtyEventData {
    Init {
        cols: usize,
        rows: usize,
        #[serde(rename = "initialData")]
        initial_data: String,
        #[serde(rename = "initialSeq")]
        initial_seq: String,
    },
    Output {
        data: String,
    },
    Resize {
        cols: usize,
        rows: usize,
    },
    Snapshot {
        cols: usize,
        rows: usize,
        seq: String,
        data: String,
    },
}

pub struct PtySession {
    terminal: Arc<PtyTerminal>,
    event_tx: broadcast::Sender<PtyEvent>,
    start_time: Instant,
    cols: usize,
    rows: usize,
}

impl PtySession {
    pub async fn new(command: String, cols: usize, rows: usize) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(1024);
        let now = Instant::now();
        let terminal = Arc::new(
            PtyTerminal::new(command, cols as u16, rows as u16, event_tx.clone(), now).await?,
        );

        let session = Self {
            terminal: terminal.clone(),
            event_tx: event_tx.clone(),
            start_time: now,
            cols,
            rows,
        };

        // No need for separate output_handler since PTY terminal emits events directly

        Ok(session)
    }

    pub async fn handle_command(&self, command: PtyCommand) -> Result<()> {
        use tracing::info;
        info!("🎯 handle_command called with: {:?}", command);

        match command {
            PtyCommand::Input { payload } => {
                info!(
                    "🔄 Processing Input command: {} bytes: {:?}",
                    payload.len(),
                    payload
                );
                self.terminal.write_input(payload.as_bytes()).await?;
                info!("✅ Input written to terminal successfully");
            }
            PtyCommand::SendKeys { keys } => {
                for key in keys {
                    let bytes = parse_key(&key);
                    self.terminal.write_input(&bytes).await?;
                }
            }
            PtyCommand::Resize { cols, rows } => {
                self.terminal.resize(cols as u16, rows as u16).await?;
                self.emit_resize_event(cols, rows).await;
            }
            PtyCommand::TakeSnapshot => {
                self.emit_snapshot_event().await?;
            }
        }
        Ok(())
    }

    pub async fn subscribe(&self) -> broadcast::Receiver<PtyEvent> {
        self.event_tx.subscribe()
    }

    async fn emit_resize_event(&self, cols: usize, rows: usize) {
        let event = PtyEvent {
            event_type: "resize".to_string(),
            time: self.get_elapsed_time().await,
            data: PtyEventData::Resize { cols, rows },
        };
        let _ = self.event_tx.send(event);
    }

    async fn emit_snapshot_event(&self) -> Result<()> {
        // Use vt100 screen dump
        let content = self.terminal.get_screen_dump().await;
        let (_cursor_x, _cursor_y) = self.terminal.get_cursor_position().await;

        let event = PtyEvent {
            event_type: "snapshot".to_string(),
            time: self.get_elapsed_time().await,
            data: PtyEventData::Snapshot {
                cols: self.cols,
                rows: self.rows,
                seq: content.clone(),
                data: content, // Return the actual screen content, not cursor info
            },
        };
        let _ = self.event_tx.send(event);
        Ok(())
    }

    async fn get_elapsed_time(&self) -> f64 {
        let elapsed = self.start_time.elapsed();
        elapsed.as_secs_f64()
    }

    /// Get properly processed screen dump from AVT terminal
    #[allow(dead_code)]
    pub async fn get_avt_terminal_output(&self) -> String {
        self.terminal.get_avt_screen_dump().await
    }

    /// Get raw ANSI output from terminal
    #[allow(dead_code)]
    pub async fn get_raw_ansi_output(&self) -> Result<Option<String>> {
        self.terminal.get_raw_ansi_output().await
    }

    /// Send output data directly to the terminal's output channel
    pub async fn send_output_data(&self, data: &str) -> Result<()> {
        use bytes::Bytes;
        self.terminal
            .write_output(Bytes::from(data.to_string()))
            .await
    }

    /// Get direct access to PTY output broadcast receiver for WebSocket streaming
    pub async fn get_pty_output_receiver(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<String>> {
        // Get a receiver for the terminal's output channel
        let mut bytes_rx = self.terminal.get_output_receiver().await?;
        let (string_tx, string_rx) = tokio::sync::broadcast::channel(1024);

        // Spawn a converter task that converts Bytes to String
        tokio::spawn(async move {
            use tracing::info;
            info!("🔄 PTY session converter task started");

            while let Ok(bytes) = bytes_rx.recv().await {
                let string_data = String::from_utf8_lossy(&bytes).to_string();
                info!(
                    "🔄 Converting {} bytes to string: {:?}",
                    bytes.len(),
                    &string_data[..std::cmp::min(100, string_data.len())]
                );

                if string_tx.send(string_data.clone()).is_err() {
                    info!("❌ Converter task: No more string receivers, stopping");
                    break;
                }
                info!("✅ Converter task: String data sent to channel");
            }
            info!("🔚 PTY session converter task terminated");
        });

        Ok(string_rx)
    }
}

fn parse_key(key: &str) -> Vec<u8> {
    let bytes: &[u8] = match key {
        "C-@" | "C-Space" | "^@" => b"\x00",
        "C-[" | "Escape" | "^[" => b"\x1b",
        "C-\\" | "^\\" => b"\x1c",
        "C-]" | "^]" => b"\x1d",
        "C-^" | "C-/" => b"\x1e",
        "C--" | "C-_" => b"\x1f",
        "Tab" => b"\x09",
        "Enter" => b"\x0d",
        "Space" => b" ",
        "Left" => b"\x1b[D",
        "Right" => b"\x1b[C",
        "Up" => b"\x1b[A",
        "Down" => b"\x1b[B",
        "C-Left" => b"\x1b[1;5D",
        "C-Right" => b"\x1b[1;5C",
        "C-Up" => b"\x1b[1;5A",
        "C-Down" => b"\x1b[1;5B",
        "Home" => b"\x1b[H",
        "End" => b"\x1b[F",
        "PageUp" => b"\x1b[5~",
        "PageDown" => b"\x1b[6~",
        "Insert" => b"\x1b[2~",
        "Delete" => b"\x1b[3~",
        "F1" => b"\x1bOP",
        "F2" => b"\x1bOQ",
        "F3" => b"\x1bOR",
        "F4" => b"\x1bOS",
        "F5" => b"\x1b[15~",
        "F6" => b"\x1b[17~",
        "F7" => b"\x1b[18~",
        "F8" => b"\x1b[19~",
        "F9" => b"\x1b[20~",
        "F10" => b"\x1b[21~",
        "F11" => b"\x1b[23~",
        "F12" => b"\x1b[24~",
        _ => {
            if let Some(ctrl_char) = parse_ctrl_key(key) {
                return vec![ctrl_char];
            } else {
                return key.as_bytes().to_vec();
            }
        }
    };
    bytes.to_vec()
}

fn parse_ctrl_key(key: &str) -> Option<u8> {
    if key.starts_with("C-") || key.starts_with("^") {
        let ch = if key.starts_with("C-") {
            key.chars().nth(2)?
        } else {
            key.chars().nth(1)?
        };

        if ch.is_ascii_lowercase() {
            Some((ch as u8) - b'a' + 1)
        } else if ch.is_ascii_uppercase() {
            Some((ch as u8) - b'A' + 1)
        } else {
            None
        }
    } else {
        None
    }
}
