use super::{
    CommandResult, TerminalBackend, TerminalBackendError, TerminalBackendResult, TerminalSnapshot,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::Mutex;

pub struct DirectTerminalBackend {
    working_directory: Mutex<PathBuf>,
    shell: String,
}

impl DirectTerminalBackend {
    pub fn new() -> Self {
        let shell = env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        Self {
            working_directory: Mutex::new(
                env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            ),
            shell,
        }
    }

    pub fn with_shell(shell: String) -> Self {
        Self {
            working_directory: Mutex::new(
                env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            ),
            shell,
        }
    }

    fn create_snapshot_from_output(output: &str, width: u32, height: u32) -> TerminalSnapshot {
        TerminalSnapshot {
            content: output.to_string(),
            cursor_position: None, // Direct execution doesn't track cursor
            width,
            height,
        }
    }
}

impl Default for DirectTerminalBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TerminalBackend for DirectTerminalBackend {
    async fn execute_command(&self, command: &str) -> TerminalBackendResult<CommandResult> {
        let working_dir = self.working_directory.lock().await.clone();

        let mut cmd = if cfg!(windows) {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", command]);
            cmd
        } else {
            let mut cmd = Command::new(&self.shell);
            cmd.args(["-c", command]);
            cmd
        };

        cmd.current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await.map_err(|e| {
            TerminalBackendError::ExecutionError(format!("Failed to execute command: {}", e))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined_output = if stderr.is_empty() {
            stdout.clone()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        let snapshot = Self::create_snapshot_from_output(&combined_output, 80, 24);

        Ok(CommandResult {
            exit_code: output.status.code(),
            output: stdout,
            error: stderr,
            snapshot: Some(snapshot),
        })
    }

    async fn send_keys(&self, _keys: &str) -> TerminalBackendResult<()> {
        // Direct backend doesn't support interactive key sending
        Err(TerminalBackendError::BackendUnavailable(
            "Direct backend does not support interactive key sending".to_string(),
        ))
    }

    async fn take_snapshot(&self) -> TerminalBackendResult<TerminalSnapshot> {
        // Direct backend doesn't have a persistent terminal to snapshot
        // Return a simple snapshot showing the current working directory
        let working_dir = self.working_directory.lock().await.clone();
        let content = format!("Current directory: {}", working_dir.display());

        Ok(TerminalSnapshot {
            content,
            cursor_position: None,
            width: 80,
            height: 24,
        })
    }

    async fn resize(&self, _width: u32, _height: u32) -> TerminalBackendResult<()> {
        // Direct backend doesn't have a resizable terminal
        Ok(()) // No-op for direct backend
    }

    async fn is_available(&self) -> bool {
        // Direct backend is always available
        true
    }

    fn backend_type(&self) -> &'static str {
        "direct"
    }

    async fn get_environment(&self) -> TerminalBackendResult<HashMap<String, String>> {
        Ok(env::vars().collect())
    }

    async fn set_working_directory(&self, path: &str) -> TerminalBackendResult<()> {
        let new_path = PathBuf::from(path);

        if !new_path.exists() {
            return Err(TerminalBackendError::InvalidCommand(format!(
                "Directory does not exist: {}",
                path
            )));
        }

        if !new_path.is_dir() {
            return Err(TerminalBackendError::InvalidCommand(format!(
                "Path is not a directory: {}",
                path
            )));
        }

        let mut working_dir = self.working_directory.lock().await;
        *working_dir = new_path;
        Ok(())
    }

    async fn cleanup(&self) -> TerminalBackendResult<()> {
        // Direct backend doesn't require cleanup
        Ok(())
    }
}
