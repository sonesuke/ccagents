//! Minimal queue manager stub for maintaining API compatibility

use serde::{Deserialize, Serialize};

/// Minimal queue manager stub
///
/// This is kept as a stub for maintaining API compatibility after simplifying
/// the queue system. The new system uses direct source processing instead.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct QueueManager;

impl QueueManager {
    /// Create a new queue manager stub
    pub fn new() -> Self {
        Self
    }
}
