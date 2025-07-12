pub mod agent_pool;
pub mod agents;
pub mod execution;
pub mod monitor;
pub mod pty_processor;
pub mod rule_engine;
pub mod terminal_agent;
pub mod timeout;

// Re-export for convenience
pub use agent_pool::AgentPool;
pub use agents::Agents;
pub use monitor::AgentMonitor;
pub use terminal_agent::{Agent, AgentStatus};
pub use timeout::TimeoutMonitor;

use anyhow::Result;

/// Common trait for all monitoring components
#[allow(dead_code)]
pub trait Monitor {
    async fn start_monitoring(self) -> Result<()>;
}
