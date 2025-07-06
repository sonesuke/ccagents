//! Queue system for managing command execution results and task processing
//!
//! This module provides a generic queue system that supports:
//! - Multiple named queues with FIFO ordering
//! - Event-driven queue listeners
//! - Command execution and result enqueueing

pub mod executor;
pub mod manager;

// pub use executor::QueueExecutor; // Unused after queue simplification
pub use manager::QueueManager;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared queue manager instance
pub type SharedQueueManager = Arc<RwLock<QueueManager>>;

/// Create a new shared queue manager
pub fn create_shared_manager() -> SharedQueueManager {
    Arc::new(RwLock::new(QueueManager::new()))
}
