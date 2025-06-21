pub mod agent;
pub mod ruler;

// Re-export legacy modules temporarily for backward compatibility
pub mod ht_client;

// Public API
pub use agent::ht_process::{HtProcess, HtProcessConfig};
pub use agent::terminal_monitor::{
    AgentState, MonitorConfig, MonitorError, MonitorStatistics, StateTransition,
    TerminalOutputMonitor, TerminalSnapshot,
};
pub use agent::Agent;
pub use ruler::rule_engine::{CompiledRule, RuleEngine, RuleFile};
pub use ruler::session::{SessionPersistence, SessionState, SessionStore};
pub use ruler::Ruler;
