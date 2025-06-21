use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum HtProcessError {
    #[error("HT process failed to start: {0}")]
    StartupFailure(String),
    #[error("HT process communication error: {0}")]
    CommunicationError(String),
    #[error("HT process crashed: {0}")]
    ProcessCrashed(String),
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
    #[serde(rename = "getView")]
    GetView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtResponse {
    pub view: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct HtProcessConfig {
    pub ht_binary_path: String,
    pub shell_command: Option<String>,
    pub restart_attempts: u32,
    pub restart_delay_ms: u64,
}

impl Default for HtProcessConfig {
    fn default() -> Self {
        Self {
            ht_binary_path: "ht".to_string(),
            shell_command: Some("bash".to_string()),
            restart_attempts: 3,
            restart_delay_ms: 1000,
        }
    }
}

#[derive(Debug)]
pub struct HtProcess {
    config: HtProcessConfig,
    process: Arc<Mutex<Option<Child>>>,
    sender: Arc<Mutex<Option<mpsc::UnboundedSender<HtMessage>>>>,
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<HtResponse>>>>,
    monitor_running: Arc<AtomicBool>,
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

    pub fn with_default_config() -> Self {
        Self::new(HtProcessConfig::default())
    }

    pub fn enable_auto_restart(&self) {
        self.auto_restart.store(true, Ordering::SeqCst);
    }

    pub fn disable_auto_restart(&self) {
        self.auto_restart.store(false, Ordering::SeqCst);
    }

    pub fn is_auto_restart_enabled(&self) -> bool {
        self.auto_restart.load(Ordering::SeqCst)
    }

    pub async fn start(&self) -> Result<(), HtProcessError> {
        let mut process_lock = self.process.lock().await;

        if process_lock.is_some() {
            warn!("HT process is already running");
            return Ok(());
        }

        info!("Starting HT process with config: {:?}", self.config);

        let mut command = Command::new(&self.config.ht_binary_path);

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

    pub async fn stop(&self) -> Result<(), HtProcessError> {
        // Stop the monitor first to prevent restart attempts
        self.monitor_running.store(false, Ordering::SeqCst);

        let mut process_lock = self.process.lock().await;

        if let Some(mut child) = process_lock.take() {
            info!("Stopping HT process");

            // Try graceful shutdown first
            if let Err(e) = child.kill() {
                warn!("Failed to kill HT process gracefully: {}", e);
            }

            // Wait for process to exit
            match child.wait() {
                Ok(status) => {
                    info!("HT process exited with status: {}", status);
                }
                Err(e) => {
                    error!("Error waiting for HT process to exit: {}", e);
                }
            }

            // Clear communication channels
            *self.sender.lock().await = None;
            *self.receiver.lock().await = None;
        } else {
            warn!("HT process is not running");
        }

        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let process_lock = self.process.lock().await;
        process_lock.is_some()
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
            let message = HtMessage::GetView;
            sender.send(message).map_err(|e| {
                HtProcessError::CommunicationError(format!("Failed to request view: {}", e))
            })?;

            // Wait for response
            drop(sender_lock);
            let mut receiver_lock = self.receiver.lock().await;

            if let Some(receiver) = receiver_lock.as_mut() {
                match receiver.recv().await {
                    Some(response) => response.view.ok_or_else(|| {
                        HtProcessError::CommunicationError("No view data in response".to_string())
                    }),
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

    pub async fn restart(&self) -> Result<(), HtProcessError> {
        info!("Restarting HT process");

        // Stop current process
        self.stop().await?;

        // Wait before restart
        sleep(Duration::from_millis(self.config.restart_delay_ms)).await;

        // Start new process
        self.start().await?;

        Ok(())
    }

    pub async fn restart_with_retry(&self) -> Result<(), HtProcessError> {
        let mut attempts = 0;

        while attempts < self.config.restart_attempts {
            attempts += 1;

            match self.restart().await {
                Ok(()) => {
                    info!("HT process restarted successfully on attempt {}", attempts);
                    return Ok(());
                }
                Err(e) => {
                    error!(
                        "Failed to restart HT process on attempt {}: {}",
                        attempts, e
                    );

                    if attempts < self.config.restart_attempts {
                        let delay = self.config.restart_delay_ms * (2_u64.pow(attempts - 1));
                        warn!("Retrying in {}ms...", delay);
                        sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(HtProcessError::ProcessCrashed(format!(
            "Failed to restart after {} attempts",
            self.config.restart_attempts
        )))
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
                    error!("HT process stderr: {}", line_content);
                }
                Err(e) => {
                    error!("Error reading from HT process stderr: {}", e);
                    break;
                }
            }
        }
    }

    fn start_process_monitor(&self) {
        self.monitor_running.store(true, Ordering::SeqCst);

        let process = Arc::clone(&self.process);
        let monitor_running = Arc::clone(&self.monitor_running);
        let auto_restart = Arc::clone(&self.auto_restart);
        let config = self.config.clone();

        // Create a clone of self for the monitor task
        let process_clone = HtProcess {
            config: config.clone(),
            process: Arc::clone(&self.process),
            sender: Arc::clone(&self.sender),
            receiver: Arc::clone(&self.receiver),
            monitor_running: Arc::clone(&self.monitor_running),
            auto_restart: Arc::clone(&self.auto_restart),
        };

        tokio::spawn(async move {
            info!("Starting HT process monitor");

            while monitor_running.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(1000)).await; // Check every second

                let mut needs_restart = false;

                // Check if process is still alive
                {
                    let mut process_lock = process.lock().await;
                    if let Some(child) = process_lock.as_mut() {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                error!("HT process exited unexpectedly with status: {}", status);
                                needs_restart = true;
                                *process_lock = None; // Remove the dead process
                            }
                            Ok(None) => {
                                // Process is still running
                                continue;
                            }
                            Err(e) => {
                                error!("Error checking HT process status: {}", e);
                                needs_restart = true;
                                *process_lock = None;
                            }
                        }
                    } else if auto_restart.load(Ordering::SeqCst) {
                        // Process is not running but should be
                        needs_restart = true;
                    }
                }

                if needs_restart && auto_restart.load(Ordering::SeqCst) {
                    warn!("HT process needs restart, attempting recovery...");

                    match process_clone.restart_with_retry().await {
                        Ok(()) => {
                            info!("HT process successfully restarted by monitor");
                        }
                        Err(e) => {
                            error!("Failed to restart HT process: {}", e);
                            // Continue monitoring in case manual restart happens
                        }
                    }
                }
            }

            info!("HT process monitor stopped");
        });
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
