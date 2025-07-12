pub mod agent;
pub mod agent_system;
pub mod periodic;
pub mod startup;
pub mod timeout;
pub mod trigger_system;
pub mod web_server_manager;

// Re-export for convenience
pub use agent::AgentMonitor;
pub use agent_system::AgentSystem;
pub use periodic::PeriodicTaskManager;
pub use startup::StartupTaskManager;
pub use timeout::TimeoutMonitor;
pub use trigger_system::TriggerSystem;
// pub use web_server_manager::WebServerManager; // Legacy - now integrated into Agent

use anyhow::Result;

/// Common trait for all monitoring components
#[allow(dead_code)]
pub trait Monitor {
    async fn start_monitoring(self) -> Result<()>;
}
