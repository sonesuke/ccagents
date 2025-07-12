#[allow(clippy::module_inception)]
mod agent;
pub mod agents;

// Re-export for convenience
pub use agent::{Agent, AgentStatus};
pub use agents::Agents;
