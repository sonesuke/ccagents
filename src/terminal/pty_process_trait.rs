use anyhow::Result;
use tokio::sync::broadcast;

/// Trait for PTY process operations to enable mocking
#[async_trait::async_trait]
pub trait PtyProcessTrait: Send + Sync {
    async fn send_input(&self, input: String) -> Result<(), crate::terminal::pty_process::PtyProcessError>;
    async fn get_pty_string_receiver(&self) -> Result<broadcast::Receiver<String>, crate::terminal::pty_process::PtyProcessError>;
    async fn get_child_processes(&self) -> Result<Vec<u32>, crate::terminal::pty_process::PtyProcessError>;
    async fn get_screen_contents(&self) -> Result<String, crate::terminal::pty_process::PtyProcessError>;
    async fn get_pty_bytes_receiver(&self) -> Result<broadcast::Receiver<bytes::Bytes>, crate::terminal::pty_process::PtyProcessError>;
}

/// Mock implementation for testing
pub struct MockPtyProcess {
    pub sent_inputs: std::sync::Mutex<Vec<String>>,
    pub should_fail: bool,
}

impl MockPtyProcess {
    pub fn new() -> Self {
        Self {
            sent_inputs: std::sync::Mutex::new(Vec::new()),
            should_fail: false,
        }
    }

    pub fn with_failure() -> Self {
        Self {
            sent_inputs: std::sync::Mutex::new(Vec::new()),
            should_fail: true,
        }
    }

    pub fn get_sent_inputs(&self) -> Vec<String> {
        self.sent_inputs.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl PtyProcessTrait for MockPtyProcess {
    async fn send_input(&self, input: String) -> Result<(), crate::terminal::pty_process::PtyProcessError> {
        if self.should_fail {
            return Err(crate::terminal::pty_process::PtyProcessError::CommunicationError("Mock failure".to_string()));
        }
        
        self.sent_inputs.lock().unwrap().push(input);
        Ok(())
    }

    async fn get_pty_string_receiver(&self) -> Result<broadcast::Receiver<String>, crate::terminal::pty_process::PtyProcessError> {
        let (tx, rx) = broadcast::channel(100);
        // Send some mock data for testing
        let _ = tx.send("mock output".to_string());
        Ok(rx)
    }

    async fn get_child_processes(&self) -> Result<Vec<u32>, crate::terminal::pty_process::PtyProcessError> {
        // Return empty vec to simulate idle state
        Ok(vec![])
    }

    async fn get_screen_contents(&self) -> Result<String, crate::terminal::pty_process::PtyProcessError> {
        if self.should_fail {
            return Err(crate::terminal::pty_process::PtyProcessError::CommunicationError("Mock screen contents failure".to_string()));
        }
        Ok("Mock screen contents".to_string())
    }

    async fn get_pty_bytes_receiver(&self) -> Result<broadcast::Receiver<bytes::Bytes>, crate::terminal::pty_process::PtyProcessError> {
        let (tx, rx) = broadcast::channel(100);
        // Send some mock bytes for testing
        let _ = tx.send(bytes::Bytes::from(b"mock bytes output".to_vec()));
        Ok(rx)
    }
}