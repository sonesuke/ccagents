use crate::agent::terminal_monitor::TerminalSnapshot;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub agent_id: String,
    pub working_directory: String,
    pub terminal_snapshot: TerminalSnapshot,
    pub environment_vars: HashMap<String, String>,
    pub last_command: Option<String>,
    pub timestamp: u64,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPersistence {
    pub sessions: HashMap<String, SessionState>,
    pub last_updated: u64,
}

impl Default for SessionPersistence {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

pub struct SessionStore {
    persistence_file: PathBuf,
    sessions: HashMap<String, SessionState>,
}

impl SessionStore {
    pub fn new<P: AsRef<Path>>(persistence_file: P) -> Self {
        Self {
            persistence_file: persistence_file.as_ref().to_path_buf(),
            sessions: HashMap::new(),
        }
    }

    pub async fn load_sessions(&mut self) -> Result<()> {
        if !self.persistence_file.exists() {
            info!("No existing session file found, starting with empty sessions");
            return Ok(());
        }

        let content = fs::read_to_string(&self.persistence_file).await?;
        let persistence: SessionPersistence = serde_json::from_str(&content)?;

        self.sessions = persistence.sessions;
        info!(
            "Loaded {} sessions from persistence file",
            self.sessions.len()
        );

        Ok(())
    }

    pub async fn save_sessions(&self) -> Result<()> {
        let persistence = SessionPersistence {
            sessions: self.sessions.clone(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let content = serde_json::to_string_pretty(&persistence)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.persistence_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.persistence_file, content).await?;
        debug!("Saved {} sessions to persistence file", self.sessions.len());

        Ok(())
    }

    pub async fn save_session_state(
        &mut self,
        agent_id: &str,
        working_directory: &str,
        terminal_snapshot: TerminalSnapshot,
        environment_vars: HashMap<String, String>,
        last_command: Option<String>,
    ) -> Result<()> {
        let session_id = format!(
            "{}_{}",
            agent_id,
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let session_state = SessionState {
            agent_id: agent_id.to_string(),
            working_directory: working_directory.to_string(),
            terminal_snapshot,
            environment_vars,
            last_command,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            session_id: session_id.clone(),
        };

        self.sessions.insert(agent_id.to_string(), session_state);
        self.save_sessions().await?;

        info!("Saved session state for agent: {}", agent_id);
        Ok(())
    }

    pub fn get_session_state(&self, agent_id: &str) -> Option<&SessionState> {
        self.sessions.get(agent_id)
    }

    pub fn get_latest_session_for_agent(&self, agent_id: &str) -> Option<&SessionState> {
        self.sessions
            .values()
            .filter(|session| session.agent_id == agent_id)
            .max_by_key(|session| session.timestamp)
    }

    pub fn list_sessions(&self) -> Vec<&SessionState> {
        self.sessions.values().collect()
    }

    pub async fn cleanup_old_sessions(&mut self, max_age_hours: u64) -> Result<()> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let max_age_seconds = max_age_hours * 3600;
        let mut removed_count = 0;

        self.sessions.retain(|agent_id, session| {
            let is_old = current_time - session.timestamp > max_age_seconds;
            if is_old {
                debug!("Removing old session for agent: {}", agent_id);
                removed_count += 1;
            }
            !is_old
        });

        if removed_count > 0 {
            self.save_sessions().await?;
            info!("Cleaned up {} old sessions", removed_count);
        }

        Ok(())
    }

    pub async fn remove_session(&mut self, agent_id: &str) -> Result<bool> {
        if self.sessions.remove(agent_id).is_some() {
            self.save_sessions().await?;
            info!("Removed session for agent: {}", agent_id);
            Ok(true)
        } else {
            warn!("No session found for agent: {}", agent_id);
            Ok(false)
        }
    }

    pub fn has_session(&self, agent_id: &str) -> bool {
        self.sessions.contains_key(agent_id)
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn get_session_mut(&mut self, agent_id: &str) -> Option<&mut SessionState> {
        self.sessions.get_mut(agent_id)
    }

    pub fn clear_all_sessions(&mut self) {
        self.sessions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_session_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut session_store = SessionStore::new(temp_file.path());

        // Create a test session state
        let snapshot = TerminalSnapshot {
            content: "test content".to_string(),
            cursor_position: Some((10, 20)),
            width: 80,
            height: 24,
        };

        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

        // Save session state
        session_store
            .save_session_state(
                "test-agent",
                "/tmp/test",
                snapshot.clone(),
                env_vars.clone(),
                Some("echo test".to_string()),
            )
            .await
            .unwrap();

        // Create new session store and load
        let mut new_session_store = SessionStore::new(temp_file.path());
        new_session_store.load_sessions().await.unwrap();

        // Verify session was loaded
        let loaded_session = new_session_store.get_session_state("test-agent").unwrap();
        assert_eq!(loaded_session.agent_id, "test-agent");
        assert_eq!(loaded_session.working_directory, "/tmp/test");
        assert_eq!(loaded_session.terminal_snapshot.content, "test content");
        assert_eq!(loaded_session.last_command, Some("echo test".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_old_sessions() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut session_store = SessionStore::new(temp_file.path());

        // Create an old session by manually setting timestamp
        let snapshot = TerminalSnapshot {
            content: "old content".to_string(),
            cursor_position: None,
            width: 80,
            height: 24,
        };

        let old_session = SessionState {
            agent_id: "old-agent".to_string(),
            working_directory: "/tmp".to_string(),
            terminal_snapshot: snapshot,
            environment_vars: HashMap::new(),
            last_command: None,
            timestamp: 0, // Very old timestamp
            session_id: "old_session".to_string(),
        };

        session_store
            .sessions
            .insert("old-agent".to_string(), old_session);

        // Cleanup sessions older than 1 hour
        session_store.cleanup_old_sessions(1).await.unwrap();

        // Verify old session was removed
        assert!(!session_store.has_session("old-agent"));
    }
}
