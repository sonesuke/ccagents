use super::{BackendType, TerminalBackendError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalBackendConfig {
    pub backend_type: BackendType,
    pub ht_config: HtBackendConfig,
    pub direct_config: DirectBackendConfig,
    pub fallback_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtBackendConfig {
    pub ht_binary_path: PathBuf,
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectBackendConfig {
    pub shell: Option<String>,
    pub timeout: Duration,
}

impl Default for TerminalBackendConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Direct,
            ht_config: HtBackendConfig::default(),
            direct_config: DirectBackendConfig::default(),
            fallback_enabled: true,
        }
    }
}

impl Default for HtBackendConfig {
    fn default() -> Self {
        Self {
            ht_binary_path: PathBuf::from("ht"),
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

impl Default for DirectBackendConfig {
    fn default() -> Self {
        Self {
            shell: None, // Will use system default
            timeout: Duration::from_secs(60),
        }
    }
}

impl TerminalBackendConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_backend_type(mut self, backend_type: BackendType) -> Self {
        self.backend_type = backend_type;
        self
    }

    pub fn with_ht_binary_path(mut self, path: PathBuf) -> Self {
        self.ht_config.ht_binary_path = path;
        self
    }

    pub fn with_ht_timeout(mut self, timeout: Duration) -> Self {
        self.ht_config.timeout = timeout;
        self
    }

    pub fn with_direct_shell(mut self, shell: String) -> Self {
        self.direct_config.shell = Some(shell);
        self
    }

    pub fn with_direct_timeout(mut self, timeout: Duration) -> Self {
        self.direct_config.timeout = timeout;
        self
    }

    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    pub fn validate(&self) -> Result<(), TerminalBackendError> {
        match self.backend_type {
            BackendType::Ht => {
                if !self.ht_config.ht_binary_path.exists() {
                    return Err(TerminalBackendError::BackendUnavailable(format!(
                        "HT binary not found at path: {}",
                        self.ht_config.ht_binary_path.display()
                    )));
                }
            }
            BackendType::Direct => {
                // Direct backend is always valid
            }
        }
        Ok(())
    }

    pub fn from_env() -> Result<Self, TerminalBackendError> {
        let mut config = Self::default();

        // Read backend type from environment
        if let Ok(backend_str) = std::env::var("TERMINAL_BACKEND") {
            config.backend_type = backend_str.parse()?;
        }

        // Read HT binary path from environment
        if let Ok(ht_path) = std::env::var("HT_BINARY_PATH") {
            config.ht_config.ht_binary_path = PathBuf::from(ht_path);
        }

        // Read shell from environment
        if let Ok(shell) = std::env::var("TERMINAL_SHELL") {
            config.direct_config.shell = Some(shell);
        }

        // Read fallback setting from environment
        if let Ok(fallback_str) = std::env::var("TERMINAL_BACKEND_FALLBACK") {
            config.fallback_enabled = fallback_str.parse().unwrap_or(true);
        }

        Ok(config)
    }
}
