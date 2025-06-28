use anyhow::{Context, Result};
use avt::Vt;
use bytes::Bytes;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

const READ_BUF_SIZE: usize = 4096;

pub struct PtyTerminal {
    master_pty: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    reader_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    writer_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    child_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    input_tx: mpsc::UnboundedSender<Bytes>,
    output_rx: Arc<Mutex<mpsc::UnboundedReceiver<Bytes>>>,
    terminal: Arc<Mutex<vt100::Parser>>,
    avt_terminal: Arc<Mutex<Vt>>,
    #[allow(dead_code)]
    raw_output_buffer: Arc<Mutex<String>>,
}

impl PtyTerminal {
    pub async fn new(command: String, cols: u16, rows: u16) -> Result<Self> {
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

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn command")?;

        drop(pair.slave);

        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Bytes>();
        let (output_tx, output_rx) = mpsc::unbounded_channel();

        let terminal = vt100::Parser::new(rows, cols, 0);
        let terminal = Arc::new(Mutex::new(terminal));

        // Create AVT terminal for proper ANSI handling
        let avt_terminal = Vt::builder()
            .size(cols as usize, rows as usize)
            .scrollback_limit(1000)
            .build();
        let avt_terminal = Arc::new(Mutex::new(avt_terminal));

        let raw_output_buffer = Arc::new(Mutex::new(String::new()));

        let reader = pair
            .master
            .try_clone_reader()
            .context("Failed to clone reader")?;
        let writer = pair.master.take_writer().context("Failed to take writer")?;

        let terminal_clone = terminal.clone();
        let avt_terminal_clone = avt_terminal.clone();
        let raw_buffer_clone = raw_output_buffer.clone();
        let reader_handle = tokio::spawn(async move {
            use std::io::Read;
            let mut reader = reader;
            let mut buf = vec![0u8; READ_BUF_SIZE];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = &buf[..n];

                        // Store raw output with ANSI sequences
                        let raw_str = String::from_utf8_lossy(data);
                        {
                            let mut raw_buffer = raw_buffer_clone.lock().await;
                            raw_buffer.push_str(&raw_str);
                            // Keep only the last 10KB to prevent unbounded growth
                            if raw_buffer.len() > 10240 {
                                // Find a safe character boundary to avoid splitting UTF-8 characters
                                let mut start = raw_buffer.len().saturating_sub(8192);
                                while start > 0 && !raw_buffer.is_char_boundary(start) {
                                    start -= 1;
                                }
                                *raw_buffer = raw_buffer[start..].to_string();
                            }
                        }

                        // Process through vt100 parser for structured access
                        let mut term = terminal_clone.lock().await;
                        term.process(data);
                        drop(term);

                        // Process through AVT terminal for proper ANSI handling
                        let mut avt_term = avt_terminal_clone.lock().await;
                        avt_term.feed_str(&raw_str);
                        drop(avt_term);

                        if output_tx.send(Bytes::from(data.to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from PTY: {}", e);
                        break;
                    }
                }
            }
        });

        let writer_clone = Arc::new(Mutex::new(writer));
        let writer_for_task = writer_clone.clone();

        let writer_handle = tokio::spawn(async move {
            while let Some(data) = input_rx.recv().await {
                let mut writer = writer_for_task.lock().await;
                if let Err(e) = writer.write_all(data.as_ref()) {
                    error!("Error writing to PTY: {}", e);
                    break;
                }
            }
        });

        let child_handle = tokio::spawn(async move {
            let _ = child.wait();
        });

        let pty_terminal = PtyTerminal {
            master_pty: Arc::new(Mutex::new(pair.master)),
            reader_handle: Arc::new(Mutex::new(Some(reader_handle))),
            writer_handle: Arc::new(Mutex::new(Some(writer_handle))),
            child_handle: Arc::new(Mutex::new(Some(child_handle))),
            input_tx,
            output_rx: Arc::new(Mutex::new(output_rx)),
            terminal,
            avt_terminal,
            raw_output_buffer,
        };

        Ok(pty_terminal)
    }

    pub async fn write_input(&self, data: &[u8]) -> Result<()> {
        self.input_tx
            .send(Bytes::from(data.to_vec()))
            .context("Failed to send input")?;
        Ok(())
    }

    pub async fn read_output(&self) -> Result<Option<Bytes>> {
        let mut rx = self.output_rx.lock().await;
        Ok(rx.recv().await)
    }

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

    #[allow(dead_code)]
    pub async fn get_screen_content(&self) -> Result<String> {
        let terminal = self.terminal.lock().await;
        let screen = terminal.screen();

        let content = screen.contents();
        Ok(content)
    }

    pub async fn get_cursor_position(&self) -> (u16, u16) {
        let terminal = self.terminal.lock().await;
        terminal.screen().cursor_position()
    }

    /// Get raw output with ANSI escape sequences preserved
    #[allow(dead_code)]
    pub async fn get_raw_output(&self) -> String {
        let buffer = self.raw_output_buffer.lock().await;
        buffer.clone()
    }

    /// Get properly processed screen dump using vt100 terminal
    pub async fn get_screen_dump(&self) -> String {
        let terminal = self.terminal.lock().await;
        let screen = terminal.screen();
        screen.contents()
    }

    /// Get properly processed screen dump using AVT terminal
    pub async fn get_avt_screen_dump(&self) -> String {
        let avt_terminal = self.avt_terminal.lock().await;
        avt_terminal.dump()
    }

    /// Get clean text content from AVT terminal (no ANSI codes)
    #[allow(dead_code)]
    pub async fn get_avt_text(&self) -> Vec<String> {
        let avt_terminal = self.avt_terminal.lock().await;
        avt_terminal.text()
    }
}

impl Drop for PtyTerminal {
    fn drop(&mut self) {
        // Abort all background tasks
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
        if let Ok(mut handle) = self.child_handle.try_lock() {
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
    }
}
