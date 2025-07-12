pub mod engine;
pub mod timeout;

// Re-export for convenience
pub use timeout::TimeoutMonitor;

use anyhow::Result;

/// Common trait for all monitoring components
#[allow(dead_code)]
pub trait Monitor {
    async fn start_monitoring(self) -> Result<()>;
}
