pub mod config;
pub mod direct_backend;
pub mod factory;
pub mod ht_backend;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub use config::{DirectBackendConfig, HtBackendConfig, TerminalBackendConfig};
pub use direct_backend::DirectTerminalBackend;
pub use factory::{TerminalBackendFactory, TerminalBackendManager};
pub use ht_backend::HtTerminalBackend;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSnapshot {
    pub content: String,
    pub cursor_position: Option<(u32, u32)>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub output: String,
    pub error: String,
    pub snapshot: Option<TerminalSnapshot>,
}

#[derive(Debug, Error)]
pub enum TerminalBackendError {
    #[error("Command execution failed: {0}")]
    ExecutionError(String),
    #[error("Backend not available: {0}")]
    BackendUnavailable(String),
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Timeout occurred: {0}")]
    Timeout(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type TerminalBackendResult<T> = Result<T, TerminalBackendError>;

#[async_trait]
pub trait TerminalBackend: Send + Sync {
    async fn execute_command(&self, command: &str) -> TerminalBackendResult<CommandResult>;

    async fn send_keys(&self, keys: &str) -> TerminalBackendResult<()>;

    async fn take_snapshot(&self) -> TerminalBackendResult<TerminalSnapshot>;

    async fn resize(&self, width: u32, height: u32) -> TerminalBackendResult<()>;

    async fn is_available(&self) -> bool;

    fn backend_type(&self) -> &'static str;

    async fn get_environment(&self) -> TerminalBackendResult<HashMap<String, String>>;

    async fn set_working_directory(&self, path: &str) -> TerminalBackendResult<()>;

    async fn cleanup(&self) -> TerminalBackendResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendType {
    Ht,
    Direct,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Ht => write!(f, "ht"),
            BackendType::Direct => write!(f, "direct"),
        }
    }
}

impl std::str::FromStr for BackendType {
    type Err = TerminalBackendError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ht" => Ok(BackendType::Ht),
            "direct" => Ok(BackendType::Direct),
            _ => Err(TerminalBackendError::InvalidCommand(format!(
                "Unknown backend type: {}",
                s
            ))),
        }
    }
}
