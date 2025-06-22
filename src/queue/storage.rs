//! Queue persistence to JSON files

use super::QueueManager;
use anyhow::Result;
use serde_json;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

/// Handles queue persistence to disk
#[allow(dead_code)]
pub struct QueueStorage {
    file_path: String,
}

impl QueueStorage {
    /// Create a new storage handler
    #[allow(dead_code)]
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
        }
    }

    /// Save queue manager state to file
    #[allow(dead_code)]
    pub async fn save(&self, manager: &QueueManager) -> Result<()> {
        debug!("Saving queue state to: {}", self.file_path);

        let json = serde_json::to_string_pretty(manager)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&self.file_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.file_path, json).await?;
        info!("Saved queue state to: {}", self.file_path);

        Ok(())
    }

    /// Load queue manager state from file
    #[allow(dead_code)]
    pub async fn load(&self) -> Result<QueueManager> {
        if !Path::new(&self.file_path).exists() {
            info!(
                "Queue state file not found, creating new manager: {}",
                self.file_path
            );
            return Ok(QueueManager::new());
        }

        debug!("Loading queue state from: {}", self.file_path);

        let contents = fs::read_to_string(&self.file_path).await?;
        let manager: QueueManager = serde_json::from_str(&contents)?;

        info!("Loaded queue state from: {}", self.file_path);

        Ok(manager)
    }

    /// Check if storage file exists
    #[allow(dead_code)]
    pub fn exists(&self) -> bool {
        Path::new(&self.file_path).exists()
    }

    /// Delete the storage file
    #[allow(dead_code)]
    pub async fn delete(&self) -> Result<()> {
        if self.exists() {
            fs::remove_file(&self.file_path).await?;
            info!("Deleted queue state file: {}", self.file_path);
        }
        Ok(())
    }

    /// Get the storage file path
    #[allow(dead_code)]
    pub fn file_path(&self) -> &str {
        &self.file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_queues.json");
        let storage = QueueStorage::new(file_path.to_str().unwrap());

        // Create manager with test data
        let mut manager = QueueManager::new();
        manager.enqueue("queue1", "item1".to_string()).unwrap();
        manager.enqueue("queue1", "item2".to_string()).unwrap();
        manager.enqueue("queue2", "item3".to_string()).unwrap();

        // Save to storage
        storage.save(&manager).await.unwrap();
        assert!(storage.exists());

        // Load from storage
        let loaded_manager = storage.load().await.unwrap();

        // Verify data
        assert_eq!(loaded_manager.queue_size("queue1"), 2);
        assert_eq!(loaded_manager.queue_size("queue2"), 1);
        assert!(loaded_manager.queue_exists("queue1"));
        assert!(loaded_manager.queue_exists("queue2"));
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");
        let storage = QueueStorage::new(file_path.to_str().unwrap());

        // Should return new manager
        let manager = storage.load().await.unwrap();
        assert_eq!(manager.list_queues().len(), 0);
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_queues.json");
        let storage = QueueStorage::new(file_path.to_str().unwrap());

        // Create and save
        let manager = QueueManager::new();
        storage.save(&manager).await.unwrap();
        assert!(storage.exists());

        // Delete
        storage.delete().await.unwrap();
        assert!(!storage.exists());
    }

    #[tokio::test]
    async fn test_create_parent_directories() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("queues.json");
        let storage = QueueStorage::new(file_path.to_str().unwrap());

        // Save should create parent directories
        let manager = QueueManager::new();
        storage.save(&manager).await.unwrap();
        assert!(storage.exists());
    }
}
