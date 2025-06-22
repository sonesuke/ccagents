use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{atomic::AtomicBool, Arc};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum HtProcessError {
    #[error("HT process failed to start: {0}")]
    StartupFailure(String),
    #[error("HT process communication error: {0}")]
    CommunicationError(String),
    #[error("HT process not running")]
    NotRunning,
    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HtMessage {
    #[serde(rename = "input")]
    Input { payload: String },
    #[serde(rename = "takeSnapshot")]
    TakeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HtResponse {
    View {
        view: Option<String>,
        status: String,
    },
    Snapshot {
        #[serde(rename = "type")]
        response_type: String,
        data: SnapshotData,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub seq: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Clone)]
pub struct HtProcessConfig {
    pub ht_binary_path: String,
    pub shell_command: Option<String>,
    #[allow(dead_code)]
    pub restart_attempts: u32,
    #[allow(dead_code)]
    pub restart_delay_ms: u64,
    pub port: u16,
}

impl Default for HtProcessConfig {
    fn default() -> Self {
        Self {
            ht_binary_path: "ht".to_string(),
            shell_command: Some(std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())),
            restart_attempts: 3,
            restart_delay_ms: 1000,
            port: 9999,
        }
    }
}

#[derive(Debug)]
pub struct HtProcess {
    config: HtProcessConfig,
    process: Arc<Mutex<Option<Child>>>,
    sender: Arc<Mutex<Option<mpsc::UnboundedSender<HtMessage>>>>,
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<HtResponse>>>>,
    #[allow(dead_code)]
    monitor_running: Arc<AtomicBool>,
    #[allow(dead_code)]
    auto_restart: Arc<AtomicBool>,
}

impl HtProcess {
    pub fn new(config: HtProcessConfig) -> Self {
        Self {
            config,
            process: Arc::new(Mutex::new(None)),
            sender: Arc::new(Mutex::new(None)),
            receiver: Arc::new(Mutex::new(None)),
            monitor_running: Arc::new(AtomicBool::new(false)),
            auto_restart: Arc::new(AtomicBool::new(true)),
        }
    }


    pub async fn start(&self) -> Result<(), HtProcessError> {
        let mut process_lock = self.process.lock().await;

        if process_lock.is_some() {
            warn!("HT process is already running");
            return Ok(());
        }

        info!("Starting HT process with config: {:?}", self.config);

        let shell = self.config.shell_command.as_deref().unwrap_or("unknown");
        println!("🐚 Starting HT with shell: {}", shell);
        println!(
            "🌐 HT terminal web interface available at: http://localhost:{}",
            self.config.port
        );

        let mut command = Command::new(&self.config.ht_binary_path);

        // Enable web interface on configured port
        command
            .arg("-l")
            .arg(format!("0.0.0.0:{}", self.config.port));

        // Subscribe to snapshot events for terminal output monitoring
        command.arg("--subscribe").arg("snapshot");

        if let Some(shell_cmd) = &self.config.shell_command {
            command.arg(shell_cmd);
        }

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                HtProcessError::StartupFailure(format!("Failed to spawn HT process: {}", e))
            })?;

        // Set up communication channels
        let (tx, rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        // Store stdin handle for sending commands
        let stdin = child.stdin.take().ok_or_else(|| {
            HtProcessError::StartupFailure("Failed to get stdin handle".to_string())
        })?;

        // Store stdout handle for reading responses
        let stdout = child.stdout.take().ok_or_else(|| {
            HtProcessError::StartupFailure("Failed to get stdout handle".to_string())
        })?;

        // Store stderr handle for monitoring errors
        let stderr = child.stderr.take().ok_or_else(|| {
            HtProcessError::StartupFailure("Failed to get stderr handle".to_string())
        })?;

        // Start input handler task
        let _input_task = tokio::spawn(async move {
            Self::handle_input(stdin, rx).await;
        });

        // Start output handler task
        let _output_task = tokio::spawn(async move {
            Self::handle_output(stdout, response_tx).await;
        });

        // Start error handler task
        let _error_task = tokio::spawn(async move {
            Self::handle_errors(stderr).await;
        });

        // Store the process and communication channels
        *process_lock = Some(child);
        *self.sender.lock().await = Some(tx);
        *self.receiver.lock().await = Some(response_rx);

        // Start process monitor if not already running
        // TODO: Re-enable process monitor after fixing Send trait issues
        // if !self.monitor_running.load(Ordering::SeqCst) {
        //     self.start_process_monitor();
        // }

        info!("HT process started successfully");
        Ok(())
    }


    pub async fn send_input(&self, input: String) -> Result<(), HtProcessError> {
        let sender_lock = self.sender.lock().await;

        if let Some(sender) = sender_lock.as_ref() {
            let message = HtMessage::Input { payload: input };
            sender.send(message).map_err(|e| {
                HtProcessError::CommunicationError(format!("Failed to send input: {}", e))
            })?;
            Ok(())
        } else {
            Err(HtProcessError::NotRunning)
        }
    }

    pub async fn get_view(&self) -> Result<String, HtProcessError> {
        let sender_lock = self.sender.lock().await;

        if let Some(sender) = sender_lock.as_ref() {
            // Take a snapshot to get current terminal state
            let snapshot_message = HtMessage::TakeSnapshot;
            sender.send(snapshot_message).map_err(|e| {
                HtProcessError::CommunicationError(format!("Failed to request snapshot: {}", e))
            })?;

            // Wait for response
            drop(sender_lock);
            let mut receiver_lock = self.receiver.lock().await;

            if let Some(receiver) = receiver_lock.as_mut() {
                match receiver.recv().await {
                    Some(response) => match response {
                        HtResponse::View { view, .. } => view.ok_or_else(|| {
                            HtProcessError::CommunicationError(
                                "No view data in response".to_string(),
                            )
                        }),
                        HtResponse::Snapshot { data, .. } => {
                            // Clean up terminal output by removing ANSI escape sequences
                            let cleaned = Self::clean_terminal_output(&data.seq);
                            Ok(cleaned)
                        }
                    },
                    None => Err(HtProcessError::CommunicationError(
                        "No response received".to_string(),
                    )),
                }
            } else {
                Err(HtProcessError::NotRunning)
            }
        } else {
            Err(HtProcessError::NotRunning)
        }
    }


    async fn handle_input(
        mut stdin: std::process::ChildStdin,
        mut rx: mpsc::UnboundedReceiver<HtMessage>,
    ) {
        while let Some(message) = rx.recv().await {
            match serde_json::to_string(&message) {
                Ok(json) => {
                    debug!("Sending to HT process: {}", json);
                    if let Err(e) = writeln!(stdin, "{}", json) {
                        error!("Failed to write to HT process stdin: {}", e);
                        break;
                    }
                    if let Err(e) = stdin.flush() {
                        error!("Failed to flush HT process stdin: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    }

    async fn handle_output(
        stdout: std::process::ChildStdout,
        response_tx: mpsc::UnboundedSender<HtResponse>,
    ) {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            match line {
                Ok(line_content) => {
                    debug!("Received from HT process: {}", line_content);

                    match serde_json::from_str::<HtResponse>(&line_content) {
                        Ok(response) => {
                            if let Err(e) = response_tx.send(response) {
                                error!("Failed to send response: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse HT response: {}, raw: {}", e, line_content);
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from HT process stdout: {}", e);
                    break;
                }
            }
        }
    }

    async fn handle_errors(stderr: std::process::ChildStderr) {
        let reader = BufReader::new(stderr);

        for line in reader.lines() {
            match line {
                Ok(line_content) => {
                    // Filter out normal HTTP server startup messages
                    if line_content.contains("HTTP server listening on")
                        || line_content.contains("live preview available at")
                        || line_content.contains("launching \"/bin/zsh\" in terminal")
                    {
                        debug!("HT process info: {}", line_content);
                    } else {
                        error!("HT process stderr: {}", line_content);
                    }
                }
                Err(e) => {
                    error!("Error reading from HT process stderr: {}", e);
                    break;
                }
            }
        }
    }

    pub fn clean_terminal_output(raw_output: &str) -> String {
        // Remove ANSI escape sequences including CSI sequences (\u{9b})
        let ansi_regex = regex::Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]|\x1B\[[\?]?[0-9;]*[hlm]|\x1B[>\=]|\x1B[c\d]|\x1B\][0-9];|\x1B\[[0-9A-Z]|\x1B[789]|\x1B\([AB]|\x1B\[[0-9]*[HJKfABCDGR`]|\x1B\[[0-9;]*[rW]|\x1B\[[0-9;]*H|\u{9b}[0-9;]*[a-zA-Z]|\u{9b}[\?]?[0-9;]*[hlm]").unwrap();
        let without_ansi = ansi_regex.replace_all(raw_output, "");

        // Remove control characters and non-printable characters (including \u{9b})
        let control_regex = regex::Regex::new(r"[\x00-\x1F\x7F\u{9b}]+").unwrap();
        let clean_text = control_regex.replace_all(&without_ansi, "");

        // Remove excessive whitespace and empty lines
        let lines: Vec<&str> = clean_text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();

        if lines.is_empty() {
            String::new()
        } else {
            lines.join(" ")
        }
    }
}

impl Drop for HtProcess {
    fn drop(&mut self) {
        // Ensure process is stopped when HtProcess is dropped
        if let Ok(mut process) = self.process.try_lock() {
            if let Some(mut child) = process.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
