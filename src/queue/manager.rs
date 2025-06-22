//! Queue manager for handling multiple named queues with event notifications

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, info};

/// Events emitted by the queue system
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum QueueEvent {
    /// Item was enqueued
    ItemEnqueued { queue_name: String, item: String },
    /// Item was dequeued
    ItemDequeued { queue_name: String, item: String },
    /// Queue was created
    QueueCreated { queue_name: String },
}

/// Manages multiple named queues with event notification
#[derive(Debug, Serialize, Deserialize)]
pub struct QueueManager {
    /// Named queues storing string items
    queues: HashMap<String, VecDeque<String>>,
    /// In-memory deduplication storage for dedupe_queues
    #[serde(skip)]
    dedupe_memory: HashMap<String, HashSet<String>>,
    /// Event listeners for each queue
    #[serde(skip)]
    queue_listeners: HashMap<String, Vec<UnboundedSender<String>>>,
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueManager {
    /// Create a new queue manager
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
            dedupe_memory: HashMap::new(),
            queue_listeners: HashMap::new(),
        }
    }

    /// Create a queue if it doesn't exist
    pub fn create_queue(&mut self, queue_name: &str) {
        if !self.queues.contains_key(queue_name) {
            info!("Creating queue: {}", queue_name);
            self.queues.insert(queue_name.to_string(), VecDeque::new());
        }
    }

    /// Enqueue an item to a named queue
    pub fn enqueue(&mut self, queue_name: &str, item: String) -> Result<()> {
        // Create queue if it doesn't exist
        self.create_queue(queue_name);

        // Add item to queue
        if let Some(queue) = self.queues.get_mut(queue_name) {
            debug!("Enqueuing item to {}: {}", queue_name, item);
            queue.push_back(item.clone());

            // Notify listeners
            if let Some(listeners) = self.queue_listeners.get_mut(queue_name) {
                // Remove disconnected listeners
                listeners.retain(|sender| sender.send(item.clone()).is_ok());
            }

            Ok(())
        } else {
            Err(anyhow!("Failed to access queue: {}", queue_name))
        }
    }

    /// Enqueue item with deduplication - skips if already seen
    pub fn enqueue_dedupe(&mut self, queue_name: &str, item: String) -> Result<bool> {
        // Create queue if it doesn't exist
        self.create_queue(queue_name);

        // Initialize dedupe memory for this queue if needed
        self.dedupe_memory
            .entry(queue_name.to_string())
            .or_default();

        // Check if we've seen this item before
        let dedupe_set = self.dedupe_memory.get_mut(queue_name).unwrap();
        if dedupe_set.contains(&item) {
            debug!("Skipping duplicate item in {}: {}", queue_name, item);
            return Ok(false); // Item was already seen
        }

        // Add to dedupe memory
        dedupe_set.insert(item.clone());

        // Add item to queue
        if let Some(queue) = self.queues.get_mut(queue_name) {
            debug!("Enqueuing dedupe item to {}: {}", queue_name, item);
            queue.push_back(item.clone());

            // Notify listeners
            if let Some(listeners) = self.queue_listeners.get_mut(queue_name) {
                // Remove disconnected listeners
                listeners.retain(|sender| sender.send(item.clone()).is_ok());
            }

            Ok(true) // Item was newly added
        } else {
            Err(anyhow!("Failed to access queue: {}", queue_name))
        }
    }

    /// Enqueue multiple items from a command output (line-separated)
    pub fn enqueue_lines(&mut self, queue_name: &str, output: &str) -> Result<usize> {
        let lines: Vec<String> = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        let count = lines.len();
        for line in lines {
            self.enqueue(queue_name, line)?;
        }

        Ok(count)
    }

    /// Enqueue multiple items with deduplication from a command output (line-separated)
    pub fn enqueue_lines_dedupe(&mut self, queue_name: &str, output: &str) -> Result<usize> {
        let lines: Vec<String> = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        let mut added_count = 0;
        for line in lines {
            if self.enqueue_dedupe(queue_name, line)? {
                added_count += 1;
            }
        }

        Ok(added_count)
    }

    /// Dequeue an item from a named queue
    #[allow(dead_code)]
    pub fn dequeue(&mut self, queue_name: &str) -> Option<String> {
        self.queues.get_mut(queue_name)?.pop_front().map(|item| {
            debug!("Dequeued item from {}: {}", queue_name, item);
            item
        })
    }

    /// Peek at the front item without removing it
    #[allow(dead_code)]
    pub fn peek(&self, queue_name: &str) -> Option<&String> {
        self.queues.get(queue_name)?.front()
    }

    /// Get the size of a queue
    #[allow(dead_code)]
    pub fn queue_size(&self, queue_name: &str) -> usize {
        self.queues.get(queue_name).map_or(0, |q| q.len())
    }

    /// Check if a queue exists
    #[allow(dead_code)]
    pub fn queue_exists(&self, queue_name: &str) -> bool {
        self.queues.contains_key(queue_name)
    }

    /// List all queue names
    #[allow(dead_code)]
    pub fn list_queues(&self) -> Vec<String> {
        self.queues.keys().cloned().collect()
    }

    /// Subscribe to queue events
    pub fn subscribe(&mut self, queue_name: &str) -> UnboundedReceiver<String> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Create queue if it doesn't exist
        self.create_queue(queue_name);

        // Add listener
        self.queue_listeners
            .entry(queue_name.to_string())
            .or_default()
            .push(tx);

        rx
    }

    /// Clear a specific queue
    #[allow(dead_code)]
    pub fn clear_queue(&mut self, queue_name: &str) -> Result<()> {
        if let Some(queue) = self.queues.get_mut(queue_name) {
            queue.clear();
            info!("Cleared queue: {}", queue_name);
            Ok(())
        } else {
            Err(anyhow!("Queue not found: {}", queue_name))
        }
    }

    /// Remove a queue entirely
    #[allow(dead_code)]
    pub fn remove_queue(&mut self, queue_name: &str) -> Result<()> {
        if self.queues.remove(queue_name).is_some() {
            self.queue_listeners.remove(queue_name);
            info!("Removed queue: {}", queue_name);
            Ok(())
        } else {
            Err(anyhow!("Queue not found: {}", queue_name))
        }
    }

    /// Get statistics for all queues
    #[allow(dead_code)]
    pub fn get_stats(&self) -> HashMap<String, usize> {
        self.queues
            .iter()
            .map(|(name, queue)| (name.clone(), queue.len()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_creation() {
        let mut manager = QueueManager::new();
        manager.create_queue("test");
        assert!(manager.queue_exists("test"));
        assert_eq!(manager.queue_size("test"), 0);
    }

    #[test]
    fn test_enqueue_dequeue() {
        let mut manager = QueueManager::new();

        // Enqueue items
        manager.enqueue("test", "item1".to_string()).unwrap();
        manager.enqueue("test", "item2".to_string()).unwrap();

        assert_eq!(manager.queue_size("test"), 2);

        // Dequeue items
        assert_eq!(manager.dequeue("test"), Some("item1".to_string()));
        assert_eq!(manager.dequeue("test"), Some("item2".to_string()));
        assert_eq!(manager.dequeue("test"), None);
    }

    #[test]
    fn test_enqueue_lines() {
        let mut manager = QueueManager::new();
        let output = "line1\nline2\n\nline3\n";

        let count = manager.enqueue_lines("test", output).unwrap();
        assert_eq!(count, 3);
        assert_eq!(manager.queue_size("test"), 3);

        assert_eq!(manager.dequeue("test"), Some("line1".to_string()));
        assert_eq!(manager.dequeue("test"), Some("line2".to_string()));
        assert_eq!(manager.dequeue("test"), Some("line3".to_string()));
    }

    #[test]
    fn test_peek() {
        let mut manager = QueueManager::new();
        manager.enqueue("test", "item1".to_string()).unwrap();

        assert_eq!(manager.peek("test"), Some(&"item1".to_string()));
        assert_eq!(manager.queue_size("test"), 1); // Item not removed
    }

    #[test]
    fn test_multiple_queues() {
        let mut manager = QueueManager::new();

        manager.enqueue("queue1", "a".to_string()).unwrap();
        manager.enqueue("queue2", "b".to_string()).unwrap();

        assert_eq!(manager.dequeue("queue1"), Some("a".to_string()));
        assert_eq!(manager.dequeue("queue2"), Some("b".to_string()));
    }

    #[test]
    fn test_clear_queue() {
        let mut manager = QueueManager::new();
        manager.enqueue("test", "item".to_string()).unwrap();

        manager.clear_queue("test").unwrap();
        assert_eq!(manager.queue_size("test"), 0);
        assert!(manager.queue_exists("test"));
    }

    #[test]
    fn test_remove_queue() {
        let mut manager = QueueManager::new();
        manager.create_queue("test");

        manager.remove_queue("test").unwrap();
        assert!(!manager.queue_exists("test"));
    }

    #[tokio::test]
    async fn test_subscribe() {
        let mut manager = QueueManager::new();
        let mut rx = manager.subscribe("test");

        manager.enqueue("test", "item".to_string()).unwrap();

        // Should receive the item
        assert_eq!(rx.recv().await, Some("item".to_string()));
    }
}
