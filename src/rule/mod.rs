pub mod capture;
pub mod diff_timeout;
pub mod when;

// Re-export for convenience
pub use diff_timeout::DiffTimeout;
pub use when::When;

use anyhow::Result;

/// Common trait for all monitoring components
#[allow(dead_code)]
pub trait Monitor {
    async fn start_monitoring(self) -> Result<()>;
}
