pub mod ruler;
pub mod agent;

// Re-export legacy modules temporarily for backward compatibility
pub mod ht_client;

// Public API
pub use ruler::Ruler;
pub use ruler::session::{SessionStore, SessionState, SessionPersistence};
pub use ruler::rule_engine::{RuleEngine, RuleFile, CompiledRule};
pub use agent::Agent;
pub use agent::ht_process::{HtProcess, HtProcessConfig};
pub use agent::terminal_monitor::{TerminalOutputMonitor, AgentState, MonitorConfig, MonitorError, MonitorStatistics, StateTransition, TerminalSnapshot};