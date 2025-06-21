use super::{
    config::{DirectBackendConfig, HtBackendConfig, TerminalBackendConfig},
    BackendType, DirectTerminalBackend, HtTerminalBackend, TerminalBackend, TerminalBackendError,
    TerminalBackendResult,
};
use crate::ht_client::HtClient;
use crate::ht_process::HtProcess;
use std::sync::Arc;
use tracing::{debug, info};

pub struct TerminalBackendFactory;

impl TerminalBackendFactory {
    pub async fn create_backend(
        config: &TerminalBackendConfig,
    ) -> TerminalBackendResult<Box<dyn TerminalBackend>> {
        match config.backend_type {
            BackendType::Ht => Self::create_ht_backend(&config.ht_config).await,
            BackendType::Direct => Self::create_direct_backend(&config.direct_config).await,
        }
    }

    pub async fn create_ht_backend(
        ht_config: &HtBackendConfig,
    ) -> TerminalBackendResult<Box<dyn TerminalBackend>> {
        info!("Creating HT terminal backend");
        debug!("HT binary path: {}", ht_config.ht_binary_path.display());

        // Check if HT binary exists
        if !ht_config.ht_binary_path.exists() {
            let error_msg = format!(
                "HT binary not found at path: {}",
                ht_config.ht_binary_path.display()
            );
            return Err(TerminalBackendError::BackendUnavailable(error_msg));
        }

        // Create HT process
        let ht_process_config = crate::ht_process::HtProcessConfig {
            ht_binary_path: ht_config.ht_binary_path.to_string_lossy().to_string(),
            shell_command: Some("bash".to_string()),
            restart_attempts: ht_config.retry_attempts,
            restart_delay_ms: ht_config.retry_delay.as_millis() as u64,
        };

        let ht_process = HtProcess::new(ht_process_config);

        // Create HT client
        let ht_client = Arc::new(HtClient::new(ht_process));

        // Test HT client availability
        match ht_client.start().await {
            Ok(_) => {
                info!("HT backend initialized successfully");
                let backend = HtTerminalBackend::new(ht_client).with_timeout(ht_config.timeout);
                Ok(Box::new(backend))
            }
            Err(e) => {
                let error_msg = format!("Failed to start HT client: {}", e);
                Err(TerminalBackendError::ExecutionError(error_msg))
            }
        }
    }

    pub async fn create_direct_backend(
        direct_config: &DirectBackendConfig,
    ) -> TerminalBackendResult<Box<dyn TerminalBackend>> {
        info!("Creating direct terminal backend");

        let backend = if let Some(shell) = &direct_config.shell {
            debug!("Using custom shell: {}", shell);
            DirectTerminalBackend::with_shell(shell.clone())
        } else {
            debug!("Using system default shell");
            DirectTerminalBackend::new()
        };

        info!("Direct backend initialized successfully");
        Ok(Box::new(backend))
    }

    pub async fn create_auto_backend() -> TerminalBackendResult<Box<dyn TerminalBackend>> {
        let config =
            TerminalBackendConfig::from_env().unwrap_or_else(|_| TerminalBackendConfig::default());
        Self::create_backend(&config).await
    }

    pub async fn create_direct_only() -> TerminalBackendResult<Box<dyn TerminalBackend>> {
        let config = TerminalBackendConfig::new().with_backend_type(BackendType::Direct);
        Self::create_backend(&config).await
    }
}

pub struct TerminalBackendManager {
    backend: Box<dyn TerminalBackend>,
    config: TerminalBackendConfig,
}

impl TerminalBackendManager {
    pub async fn new(config: TerminalBackendConfig) -> TerminalBackendResult<Self> {
        let backend = TerminalBackendFactory::create_backend(&config).await?;
        Ok(Self { backend, config })
    }

    pub async fn new_auto() -> TerminalBackendResult<Self> {
        let config =
            TerminalBackendConfig::from_env().unwrap_or_else(|_| TerminalBackendConfig::default());
        Self::new(config).await
    }

    pub fn backend(&self) -> &dyn TerminalBackend {
        self.backend.as_ref()
    }

    pub fn config(&self) -> &TerminalBackendConfig {
        &self.config
    }

    pub async fn switch_backend(
        &mut self,
        new_config: TerminalBackendConfig,
    ) -> TerminalBackendResult<()> {
        info!(
            "Switching terminal backend from {} to {}",
            self.backend.backend_type(),
            new_config.backend_type
        );

        // Clean up current backend
        self.backend.cleanup().await?;

        // Create new backend
        let new_backend = TerminalBackendFactory::create_backend(&new_config).await?;

        self.backend = new_backend;
        self.config = new_config;

        info!(
            "Successfully switched to {} backend",
            self.backend.backend_type()
        );
        Ok(())
    }

    pub async fn is_backend_available(&self) -> bool {
        self.backend.is_available().await
    }

    pub async fn cleanup(&self) -> TerminalBackendResult<()> {
        self.backend.cleanup().await
    }
}
