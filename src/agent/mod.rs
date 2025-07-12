#[allow(clippy::module_inception)]
mod agent;
pub mod agents;

// Re-export for convenience
pub use agent::{Agent, AgentStatus}; // TerminalSize is used internally
pub use agents::Agents;
