#[allow(clippy::module_inception)]
mod agent;
pub mod agents;

// Re-export for convenience
pub use agent::{Agent, AgentStatus, execute_entry, execute_rule_action};
pub use agents::Agents;
