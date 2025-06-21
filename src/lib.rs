pub mod ht_client;
pub mod ht_process;
pub mod manager;
pub mod rule_engine;
pub mod terminal_backend;

pub use ht_client::{HtClient, HtClientError, HtCommand, HtEvent, TerminalSnapshot};
pub use ht_process::HtProcess;
pub use manager::Manager;
pub use rule_engine::{CompiledRule, RuleEngine, RuleFile};
pub use terminal_backend::{
    BackendType, CommandResult, DirectBackendConfig, DirectTerminalBackend, HtBackendConfig,
    HtTerminalBackend, TerminalBackend, TerminalBackendConfig, TerminalBackendError,
    TerminalBackendFactory, TerminalBackendManager, TerminalBackendResult,
    TerminalSnapshot as BackendTerminalSnapshot,
};
