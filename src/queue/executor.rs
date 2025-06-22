//! Command execution and queue integration

use super::SharedQueueManager;
use anyhow::{anyhow, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info};

/// Executes commands and manages queue operations
pub struct QueueExecutor {
    manager: SharedQueueManager,
}

impl QueueExecutor {
    /// Create a new queue executor
    pub fn new(manager: SharedQueueManager) -> Self {
        Self { manager }
    }

    /// Execute a command and return its output
    pub async fn execute_command(&self, command: &str) -> Result<String> {
        debug!("Executing command: {}", command);

        // Split command into program and args
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        let program = parts[0];
        let args = &parts[1..];

        // Execute command
        let output = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            debug!("Command output: {} bytes", stdout.len());
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Command failed: {}", stderr);
            Err(anyhow!(
                "Command failed with status {}: {}",
                output.status,
                stderr
            ))
        }
    }

    /// Execute a command and enqueue its output lines to a queue
    pub async fn execute_and_enqueue(&self, queue_name: &str, command: &str) -> Result<usize> {
        info!(
            "Executing command and enqueuing to {}: {}",
            queue_name, command
        );

        let output = self.execute_command(command).await?;

        if output.trim().is_empty() {
            debug!("Command produced no output, nothing to enqueue");
            return Ok(0);
        }

        let mut manager = self.manager.write().await;
        let count = manager.enqueue_lines(queue_name, &output)?;

        info!("Enqueued {} items to queue: {}", count, queue_name);
        Ok(count)
    }

    /// Dequeue an item from a queue
    #[allow(dead_code)]
    pub async fn dequeue(&self, queue_name: &str) -> Option<String> {
        let mut manager = self.manager.write().await;
        manager.dequeue(queue_name)
    }

    /// Peek at the front item in a queue
    #[allow(dead_code)]
    pub async fn peek(&self, queue_name: &str) -> Option<String> {
        let manager = self.manager.read().await;
        manager.peek(queue_name).cloned()
    }

    /// Get queue size
    #[allow(dead_code)]
    pub async fn queue_size(&self, queue_name: &str) -> usize {
        let manager = self.manager.read().await;
        manager.queue_size(queue_name)
    }

    /// Check if queue exists
    #[allow(dead_code)]
    pub async fn queue_exists(&self, queue_name: &str) -> bool {
        let manager = self.manager.read().await;
        manager.queue_exists(queue_name)
    }

    /// Subscribe to queue events
    #[allow(dead_code)]
    pub async fn subscribe(
        &self,
        queue_name: &str,
    ) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        let mut manager = self.manager.write().await;
        manager.subscribe(queue_name)
    }

    /// Get statistics for all queues
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> std::collections::HashMap<String, usize> {
        let manager = self.manager.read().await;
        manager.get_stats()
    }

    /// Clear a specific queue
    #[allow(dead_code)]
    pub async fn clear_queue(&self, queue_name: &str) -> Result<()> {
        let mut manager = self.manager.write().await;
        manager.clear_queue(queue_name)
    }

    /// Create a queue
    #[allow(dead_code)]
    pub async fn create_queue(&self, queue_name: &str) {
        let mut manager = self.manager.write().await;
        manager.create_queue(queue_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::create_shared_manager;

    #[tokio::test]
    async fn test_execute_command() {
        let manager = create_shared_manager();
        let executor = QueueExecutor::new(manager);

        // Test successful command
        let result = executor.execute_command("echo hello").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[tokio::test]
    async fn test_execute_command_failure() {
        let manager = create_shared_manager();
        let executor = QueueExecutor::new(manager);

        // Test failing command
        let result = executor.execute_command("false").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_and_enqueue() {
        let manager = create_shared_manager();
        let executor = QueueExecutor::new(manager.clone());

        // Test enqueue directly with manager to avoid shell command complexity
        {
            let mut manager_guard = manager.write().await;
            manager_guard
                .enqueue_lines("test", "line1\nline2\nline3")
                .unwrap();
        }

        // Verify items were enqueued
        assert_eq!(executor.queue_size("test").await, 3);
        assert_eq!(executor.dequeue("test").await, Some("line1".to_string()));
        assert_eq!(executor.dequeue("test").await, Some("line2".to_string()));
        assert_eq!(executor.dequeue("test").await, Some("line3".to_string()));

        // Test with a simple command that should work across platforms
        let count = executor
            .execute_and_enqueue("test2", "echo hello")
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(executor.dequeue("test2").await, Some("hello".to_string()));
    }

    #[tokio::test]
    async fn test_execute_and_enqueue_empty_output() {
        let manager = create_shared_manager();
        let executor = QueueExecutor::new(manager);

        // Execute command with no output
        let count = executor
            .execute_and_enqueue("test", "echo -n")
            .await
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(executor.queue_size("test").await, 0);
    }

    #[tokio::test]
    async fn test_queue_operations() {
        let manager = create_shared_manager();
        let executor = QueueExecutor::new(manager);

        // Test queue creation
        executor.create_queue("test").await;
        assert!(executor.queue_exists("test").await);

        // Test manual enqueue through manager
        {
            let mut manager = executor.manager.write().await;
            manager.enqueue("test", "item1".to_string()).unwrap();
        }

        // Test peek and dequeue
        assert_eq!(executor.peek("test").await, Some("item1".to_string()));
        assert_eq!(executor.dequeue("test").await, Some("item1".to_string()));
        assert_eq!(executor.queue_size("test").await, 0);
    }

    #[tokio::test]
    async fn test_subscribe() {
        let manager = create_shared_manager();
        let executor = QueueExecutor::new(manager);

        // Subscribe to queue
        let mut rx = executor.subscribe("test").await;

        // Enqueue item
        {
            let mut manager = executor.manager.write().await;
            manager.enqueue("test", "notification".to_string()).unwrap();
        }

        // Should receive notification
        assert_eq!(rx.recv().await, Some("notification".to_string()));
    }
}
