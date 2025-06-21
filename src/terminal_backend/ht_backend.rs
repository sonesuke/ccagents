use super::{
    CommandResult, TerminalBackend, TerminalBackendError, TerminalBackendResult,
    TerminalSnapshot as BackendTerminalSnapshot,
};
use crate::ht_client::{HtClient, HtClientError, TerminalSnapshot as HtTerminalSnapshot};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct HtTerminalBackend {
    client: Arc<HtClient>,
    timeout_duration: Duration,
}

impl HtTerminalBackend {
    pub fn new(client: Arc<HtClient>) -> Self {
        Self {
            client,
            timeout_duration: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    fn convert_snapshot(ht_snapshot: HtTerminalSnapshot) -> BackendTerminalSnapshot {
        BackendTerminalSnapshot {
            content: ht_snapshot.content,
            cursor_position: Some((ht_snapshot.cursor_x, ht_snapshot.cursor_y)),
            width: ht_snapshot.width,
            height: ht_snapshot.height,
        }
    }

    fn convert_error(error: HtClientError) -> TerminalBackendError {
        match error {
            HtClientError::ProcessError(e) => TerminalBackendError::ExecutionError(e.to_string()),
            HtClientError::CommunicationError(e) => TerminalBackendError::ExecutionError(e),
            HtClientError::SerializationError(e) => TerminalBackendError::SerializationError(e),
            HtClientError::Timeout => {
                TerminalBackendError::Timeout("HT client timeout".to_string())
            }
            HtClientError::InvalidResponse(e) => TerminalBackendError::ExecutionError(e),
        }
    }
}

#[async_trait]
impl TerminalBackend for HtTerminalBackend {
    async fn execute_command(&self, command: &str) -> TerminalBackendResult<CommandResult> {
        // For HT backend, command execution is done via send_keys + snapshot
        self.send_keys(command).await?;

        // Send enter to execute the command
        self.send_keys("\r").await?;

        // Wait a bit for command to execute
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Take snapshot to get the output
        let snapshot = self.take_snapshot().await?;

        // Note: HT doesn't provide direct exit code access, so we return None
        Ok(CommandResult {
            exit_code: None,
            output: snapshot.content.clone(),
            error: String::new(),
            snapshot: Some(snapshot),
        })
    }

    async fn send_keys(&self, keys: &str) -> TerminalBackendResult<()> {
        let future = self.client.send_keys(keys);

        timeout(self.timeout_duration, future)
            .await
            .map_err(|_| TerminalBackendError::Timeout("Send keys timeout".to_string()))?
            .map_err(Self::convert_error)
    }

    async fn take_snapshot(&self) -> TerminalBackendResult<BackendTerminalSnapshot> {
        let future = self.client.take_snapshot();

        let ht_snapshot = timeout(self.timeout_duration, future)
            .await
            .map_err(|_| TerminalBackendError::Timeout("Take snapshot timeout".to_string()))?
            .map_err(Self::convert_error)?;

        Ok(Self::convert_snapshot(ht_snapshot))
    }

    async fn resize(&self, width: u32, height: u32) -> TerminalBackendResult<()> {
        let future = self.client.resize(width, height);

        timeout(self.timeout_duration, future)
            .await
            .map_err(|_| TerminalBackendError::Timeout("Resize timeout".to_string()))?
            .map_err(Self::convert_error)
    }

    async fn is_available(&self) -> bool {
        self.client.is_running().await
    }

    fn backend_type(&self) -> &'static str {
        "ht"
    }

    async fn get_environment(&self) -> TerminalBackendResult<HashMap<String, String>> {
        // Get environment variables through HT terminal
        self.send_keys("env").await?;
        self.send_keys("\r").await?;

        // Wait for command to execute
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let snapshot = self.take_snapshot().await?;

        // Parse environment variables from terminal output
        let mut env_vars = HashMap::new();
        for line in snapshot.content.lines() {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();
                if !key.is_empty() && !value.is_empty() {
                    env_vars.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(env_vars)
    }

    async fn set_working_directory(&self, path: &str) -> TerminalBackendResult<()> {
        // Change directory through HT terminal
        let cd_command = format!("cd {}", path);
        self.send_keys(&cd_command).await?;
        self.send_keys("\r").await?;

        // Wait for command to execute
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    async fn cleanup(&self) -> TerminalBackendResult<()> {
        self.client.stop().await.map_err(Self::convert_error)
    }
}
