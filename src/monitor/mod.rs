pub mod agent;
pub mod agents;
pub mod timeout;
pub mod web_server_manager;

// Re-export for convenience
pub use agent::AgentMonitor;
pub use agents::Agents;
pub use timeout::TimeoutMonitor;
// pub use web_server_manager::WebServerManager; // Legacy - now integrated into Agent

use anyhow::Result;

/// Common trait for all monitoring components
#[allow(dead_code)]
pub trait Monitor {
    async fn start_monitoring(self) -> Result<()>;
}
