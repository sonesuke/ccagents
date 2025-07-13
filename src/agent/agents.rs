use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent::Agent;
use crate::config::loader::MonitorConfig;
use crate::config::rule::Rule;

/// Agents responsible for managing agent pool and monitoring agents
pub struct Agents {
    rules: Vec<Rule>,
    agents: Vec<Arc<Agent>>,
}

impl Agents {
    /// Create a new agents system from monitor configuration
    pub async fn new(rules: Vec<Rule>, monitor_config: &MonitorConfig) -> Result<Self> {
        let pool_size = monitor_config.get_agent_pool_size();
        let mut agents = Vec::with_capacity(pool_size);

        for i in 0..pool_size {
            let agent = Agent::from_monitor_config(i, monitor_config).await?;
            agents.push(agent);
        }

        Ok(Self { rules, agents })
    }

    /// Get the number of agents in the pool
    pub fn size(&self) -> usize {
        self.agents.len()
    }

    /// Get agent by index
    pub fn get_agent_by_index(&self, index: usize) -> Arc<Agent> {
        Arc::clone(&self.agents[index % self.agents.len()])
    }

    /// Start all monitoring systems: agent monitors with timeout monitoring per agent
    pub async fn start_all(&self) -> Result<Vec<JoinHandle<()>>> {
        let mut monitoring_handles = Vec::new();

        // Setup monitoring for each agent (includes both When and DiffTimeout monitoring)
        for agent in &self.agents {
            let agent_handles = Arc::clone(agent)
                .setup_monitoring(self.rules.clone())
                .await?;
            monitoring_handles.extend(agent_handles);
        }

        Ok(monitoring_handles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rule::RuleType;
    use crate::config::types::ActionType;

    #[tokio::test]
    async fn test_agents_creation() {
        let monitor_config = MonitorConfig::default();
        let rules = vec![];

        let agents = Agents::new(rules, &monitor_config).await;
        assert!(agents.is_ok(), "Agents creation should succeed");

        let agents = agents.unwrap();
        assert_eq!(agents.size(), monitor_config.get_agent_pool_size());
    }

    #[tokio::test]
    #[ignore] // Temporarily disabled due to PTY resource conflicts
    async fn test_agents_creation_with_custom_pool_size() {
        let mut monitor_config = MonitorConfig::default();
        monitor_config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        monitor_config.agents.pool = 3;
        let rules = vec![];

        let agents = Agents::new(rules, &monitor_config).await;
        assert!(agents.is_ok(), "Agents creation should succeed");

        let agents = agents.unwrap();
        assert_eq!(agents.size(), 3);
    }

    #[tokio::test]
    #[ignore] // Temporarily disabled due to PTY resource conflicts
    async fn test_agents_size() {
        let mut monitor_config = MonitorConfig::default();
        monitor_config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        monitor_config.agents.pool = 5;
        let rules = vec![];

        let agents = Agents::new(rules, &monitor_config).await.unwrap();
        assert_eq!(agents.size(), 5);
    }

    #[tokio::test]
    #[ignore] // Temporarily disabled due to PTY resource conflicts
    async fn test_get_agent_by_index() {
        let mut monitor_config = MonitorConfig::default();
        monitor_config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        monitor_config.agents.pool = 3;
        let rules = vec![];

        let agents = Agents::new(rules, &monitor_config).await.unwrap();

        // Test getting agents by valid indices
        let agent0 = agents.get_agent_by_index(0);
        let agent1 = agents.get_agent_by_index(1);
        let agent2 = agents.get_agent_by_index(2);

        assert_eq!(agent0.get_id(), "agent-0");
        assert_eq!(agent1.get_id(), "agent-1");
        assert_eq!(agent2.get_id(), "agent-2");

        // Test wrapping behavior - index 3 should wrap to 0
        let agent3 = agents.get_agent_by_index(3);
        assert_eq!(agent3.get_id(), "agent-0");

        // Test wrapping behavior - index 4 should wrap to 1
        let agent4 = agents.get_agent_by_index(4);
        assert_eq!(agent4.get_id(), "agent-1");
    }

    #[tokio::test]
    async fn test_start_all_with_empty_rules() {
        let mut monitor_config = MonitorConfig::default();
        monitor_config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        let rules = vec![];

        let agents = Agents::new(rules, &monitor_config).await.unwrap();
        let result = agents.start_all().await;

        assert!(result.is_ok(), "start_all should succeed with empty rules");

        let handles = result.unwrap();
        // Should have 3 handles per agent (status monitoring, when monitoring, diff_timeout monitoring)
        let expected_handles = monitor_config.get_agent_pool_size() * 3;
        assert_eq!(handles.len(), expected_handles);

        // Clean up by aborting all handles
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn test_start_all_with_rules() {
        use regex::Regex;

        let mut monitor_config = MonitorConfig::default();
        monitor_config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        let rules = vec![
            Rule {
                rule_type: RuleType::When(Regex::new("test").unwrap()),
                action: ActionType::SendKeys(vec!["echo".to_string()]),
            },
            Rule {
                rule_type: RuleType::DiffTimeout(std::time::Duration::from_secs(1)),
                action: ActionType::SendKeys(vec!["timeout".to_string()]),
            },
        ];

        let agents = Agents::new(rules, &monitor_config).await.unwrap();
        let result = agents.start_all().await;

        assert!(result.is_ok(), "start_all should succeed with rules");

        let handles = result.unwrap();
        // Should have 3 handles per agent (status monitoring, when monitoring, diff_timeout monitoring)
        let expected_handles = monitor_config.get_agent_pool_size() * 3;
        assert_eq!(handles.len(), expected_handles);

        // Clean up by aborting all handles
        for handle in handles {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn test_agents_with_single_agent() {
        let mut monitor_config = MonitorConfig::default();
        monitor_config.web_ui.enabled = false; // Disable WebUI to avoid port conflicts
        monitor_config.agents.pool = 1;
        let rules = vec![];

        let agents = Agents::new(rules, &monitor_config).await.unwrap();
        assert_eq!(agents.size(), 1);

        // Test that wrapping works correctly with single agent
        let agent0 = agents.get_agent_by_index(0);
        let agent1 = agents.get_agent_by_index(1);
        let agent10 = agents.get_agent_by_index(10);

        assert_eq!(agent0.get_id(), "agent-0");
        assert_eq!(agent1.get_id(), "agent-0"); // Should wrap to 0
        assert_eq!(agent10.get_id(), "agent-0"); // Should wrap to 0
    }
}
