//! Queue system for managing command execution results and task processing
//!
//! This module provides a generic queue system that supports:
//! - Multiple named queues with FIFO ordering
//! - Event-driven queue listeners
//! - Command execution and result enqueueing
//! - Persistence to disk

pub mod executor;
pub mod manager;
pub mod storage;

pub use executor::QueueExecutor;
pub use manager::QueueManager;
pub use storage::QueueStorage;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared queue manager instance
pub type SharedQueueManager = Arc<RwLock<QueueManager>>;

/// Create a new shared queue manager
pub fn create_shared_manager() -> SharedQueueManager {
    Arc::new(RwLock::new(QueueManager::new()))
}

/// Load queue manager from storage
#[allow(dead_code)]
pub async fn load_from_storage(path: &str) -> Result<SharedQueueManager> {
    let storage = QueueStorage::new(path);
    let manager = storage.load().await?;
    Ok(Arc::new(RwLock::new(manager)))
}

/// Save queue manager to storage
#[allow(dead_code)]
pub async fn save_to_storage(manager: &SharedQueueManager, path: &str) -> Result<()> {
    let storage = QueueStorage::new(path);
    let manager_guard = manager.read().await;
    storage.save(&manager_guard).await
}
