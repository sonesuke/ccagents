use crate::agent::pty_session::{PtyEvent, PtyEventData};
use anyhow::{Context, Result};
use bytes::Bytes;
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{error, info};

const READ_BUF_SIZE: usize = 4096;

pub struct PtyTerminal {
    master_pty: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    child_process: Arc<Mutex<Option<Box<dyn Child + Send + Sync>>>>,
    reader_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    writer_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    input_tx: mpsc::UnboundedSender<Bytes>,
    // Raw bytes output for WebSocket (asciinema)
    output_tx: broadcast::Sender<Bytes>,
    // String output for rule matching
    string_output_tx: broadcast::Sender<String>,

    _persistent_rx: broadcast::Receiver<Bytes>,
    _persistent_string_rx: broadcast::Receiver<String>,
    terminal: Arc<Mutex<vt100::Parser>>,
    // Accumulated terminal state for initial WebSocket sync
    accumulated_output: Arc<Mutex<Vec<u8>>>,
}

impl PtyTerminal {
    pub async fn new(
        command: String,
        cols: u16,
        rows: u16,
        event_tx: broadcast::Sender<PtyEvent>,
        start_time: Instant,
    ) -> Result<Self> {
        info!("Creating native terminal with size {}x{}", cols, rows);

        let pty_system = NativePtySystem::default();
        let pty_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(pty_size)
            .context("Failed to create PTY")?;

        let mut cmd = CommandBuilder::new_default_prog();
        cmd.env("TERM", "xterm-256color");

        // Set current working directory to the project root
        if let Ok(current_dir) = std::env::current_dir() {
            cmd.cwd(current_dir);
        }

        let parts: Vec<&str> = command.split_whitespace().collect();
        if !parts.is_empty() {
            cmd = CommandBuilder::new(parts[0]);
            for arg in &parts[1..] {
                cmd.arg(arg);
            }
            cmd.env("TERM", "xterm-256color");

            // Set current working directory for the command as well
            if let Ok(current_dir) = std::env::current_dir() {
                cmd.cwd(current_dir);
            }
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn command")?;

        drop(pair.slave);

        info!("✅ Shell process spawned successfully");

        // Give the shell a moment to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Bytes>();
        let (output_tx, _rx) = broadcast::channel(1024);
        let (string_output_tx, _string_rx) = broadcast::channel(1024);

        // Keep persistent receivers alive to prevent broadcast channels from failing
        let persistent_rx = output_tx.subscribe();
        let persistent_string_rx = string_output_tx.subscribe();

        let terminal = vt100::Parser::new(rows, cols, 0);
        let terminal = Arc::new(Mutex::new(terminal));

        let accumulated_output = Arc::new(Mutex::new(Vec::new()));

        let reader = pair
            .master
            .try_clone_reader()
            .context("Failed to clone reader")?;
        let writer = pair.master.take_writer().context("Failed to take writer")?;

        let terminal_clone = terminal.clone();
        let output_tx_clone = output_tx.clone();
        let string_output_tx_clone = string_output_tx.clone();
        let event_tx_clone = event_tx.clone();
        let accumulated_output_clone = accumulated_output.clone();
        let reader_handle = tokio::spawn(async move {
            use std::io::Read;
            let mut reader = reader;
            let mut buf = vec![0u8; READ_BUF_SIZE];

            info!("🔍 PTY reader task started, entering read loop");

            loop {
                info!("🔄 PTY reader: attempting to read from PTY...");
                match reader.read(&mut buf) {
                    Ok(0) => {
                        info!("🔚 PTY reader: read 0 bytes, EOF reached, breaking");
                        break;
                    }
                    Ok(n) => {
                        let data = &buf[..n];
                        info!(
                            "📥 PTY reader: read {} bytes from PTY: {:?}",
                            n,
                            String::from_utf8_lossy(data)
                        );

                        // Process through vt100 parser for structured access
                        let mut term = terminal_clone.lock().await;
                        term.process(data);
                        drop(term);

                        // Accumulate output for initial state
                        {
                            let mut accumulated = accumulated_output_clone.lock().await;
                            accumulated.extend_from_slice(data);
                            // Keep reasonable size limit (100KB)
                            if accumulated.len() > 102400 {
                                let start = accumulated.len().saturating_sub(81920);
                                accumulated.drain(..start);
                            }
                        }

                        let raw_str = String::from_utf8_lossy(data);

                        info!(
                            "📤 PTY reader: broadcasting {} bytes to output channel",
                            data.len()
                        );
                        // Send raw bytes to WebSocket channel
                        if output_tx_clone.send(Bytes::from(data.to_vec())).is_err() {
                            error!(
                                "❌ PTY reader: failed to broadcast to output channel, breaking"
                            );
                            break;
                        }
                        info!("✅ PTY reader: successfully broadcast to output channel");

                        // Send string to rule matching channel
                        if string_output_tx_clone.send(raw_str.to_string()).is_err() {
                            error!(
                                "❌ PTY reader: failed to broadcast to string output channel, breaking"
                            );
                            break;
                        }
                        info!("✅ PTY reader: successfully broadcast to string output channel");

                        // Also emit the output event directly
                        let output_event = PtyEvent {
                            event_type: "output".to_string(),
                            time: start_time.elapsed().as_secs_f64(),
                            data: PtyEventData::Output {
                                data: raw_str.to_string(),
                            },
                        };

                        info!(
                            "📡 PTY reader: emitting output event with {} bytes",
                            raw_str.len()
                        );
                        if event_tx_clone.send(output_event).is_err() {
                            error!("❌ PTY reader: failed to send output event, breaking");
                            break;
                        }
                        info!("✅ PTY reader: successfully emitted output event");
                    }
                    Err(e) => {
                        error!("❌ PTY reader: Error reading from PTY: {}", e);
                        // Add a small delay before breaking to see if this is a temporary issue
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        break;
                    }
                }
            }
            info!("🔚 PTY reader task terminating");
        });

        let writer_clone = Arc::new(Mutex::new(writer));
        let writer_for_task = writer_clone.clone();

        let writer_handle = tokio::spawn(async move {
            info!("🔍 PTY writer task started");
            while let Some(data) = input_rx.recv().await {
                info!(
                    "📝 PTY writer: Writing {} bytes to PTY: {:?}",
                    data.len(),
                    String::from_utf8_lossy(&data)
                );
                let mut writer = writer_for_task.lock().await;
                if let Err(e) = writer.write_all(data.as_ref()) {
                    error!("❌ PTY writer: Error writing to PTY: {}", e);
                    break;
                } else {
                    info!("✅ PTY writer: Data written to PTY, flushing...");
                    // Flush to ensure data is sent immediately
                    if let Err(e) = writer.flush() {
                        error!("❌ PTY writer: Error flushing PTY: {}", e);
                        break;
                    } else {
                        info!("✅ PTY writer: Successfully flushed PTY");
                    }
                }
            }
            info!("🔚 PTY writer task terminating");
        });

        // Store child process to keep it alive
        let child_process = Arc::new(Mutex::new(Some(child)));

        let pty_terminal = PtyTerminal {
            master_pty: Arc::new(Mutex::new(pair.master)),
            child_process,
            reader_handle: Arc::new(Mutex::new(Some(reader_handle))),
            writer_handle: Arc::new(Mutex::new(Some(writer_handle))),
            input_tx,
            output_tx,
            string_output_tx,
            _persistent_rx: persistent_rx,
            _persistent_string_rx: persistent_string_rx,
            terminal,
            accumulated_output,
        };

        Ok(pty_terminal)
    }

    pub async fn write_input(&self, data: &[u8]) -> Result<()> {
        self.input_tx
            .send(Bytes::from(data.to_vec()))
            .context("Failed to send input")?;
        Ok(())
    }

    /// Write output data directly to the output channel (for external data injection)
    pub async fn write_output(&self, data: Bytes) -> Result<()> {
        self.output_tx.send(data).context("Failed to send output")?;
        Ok(())
    }

    /// Get a new broadcast receiver for raw bytes output (for WebSocket/asciinema)
    pub async fn get_output_receiver(&self) -> Result<broadcast::Receiver<Bytes>> {
        Ok(self.output_tx.subscribe())
    }

    /// Get a new broadcast receiver for string output (for rule matching)
    pub async fn get_string_output_receiver(&self) -> Result<broadcast::Receiver<String>> {
        Ok(self.string_output_tx.subscribe())
    }

    /// Resize the terminal
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let master = self.master_pty.lock().await;
        let pty_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        master.resize(pty_size).context("Failed to resize PTY")?;

        let mut terminal = self.terminal.lock().await;
        terminal.set_size(rows, cols);

        Ok(())
    }

    /// Get accumulated terminal output for initial WebSocket state
    pub async fn get_accumulated_output(&self) -> Vec<u8> {
        let accumulated = self.accumulated_output.lock().await;
        accumulated.clone()
    }
}

impl Drop for PtyTerminal {
    fn drop(&mut self) {
        // Properly terminate child process first
        if let Ok(mut child_guard) = self.child_process.try_lock() {
            if let Some(mut child) = child_guard.take() {
                info!("🔄 Terminating child process gracefully");
                // Try to kill the child process gracefully
                if let Err(e) = child.kill() {
                    error!("Failed to kill child process: {}", e);
                }
                // Wait for it to exit
                if let Err(e) = child.wait() {
                    error!("Failed to wait for child process: {}", e);
                }
                info!("✅ Child process terminated");
            }
        }

        // Abort reader/writer tasks
        if let Ok(mut handle) = self.reader_handle.try_lock() {
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
        if let Ok(mut handle) = self.writer_handle.try_lock() {
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
    }
}
