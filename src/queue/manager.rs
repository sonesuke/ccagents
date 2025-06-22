//! Queue manager for handling multiple named queues with event notifications

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, info};

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
}
