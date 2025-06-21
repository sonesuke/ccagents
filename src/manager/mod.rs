use crate::rule_engine::{decide_cmd, load_rules, CmdKind, CompiledRule};
use anyhow::Result;

#[derive(Debug)]
pub struct Manager {
    rules: Vec<CompiledRule>,
}

impl Manager {
    pub async fn new(rules_path: &str) -> Result<Self> {
        let rules = load_rules(std::path::Path::new(rules_path))?;
        Ok(Manager { rules })
    }

    pub async fn handle_waiting_state(&self, agent_id: &str, capture: &str) -> Result<()> {
        let (command, args) = decide_cmd(capture, &self.rules);

        println!(
            "Agent {}: Capture \"{}\" → {:?} {:?}",
            agent_id, capture, command, args
        );

        self.send_command_to_agent(agent_id, command, args).await
    }

    async fn send_command_to_agent(
        &self,
        agent_id: &str,
        command: CmdKind,
        args: Vec<String>,
    ) -> Result<()> {
        match command {
            CmdKind::SolveIssue => {
                println!(
                    "→ Sending solve-issue to agent {} with args: {:?}",
                    agent_id, args
                );
            }
            CmdKind::Cancel => {
                println!("→ Sending cancel to agent {}", agent_id);
            }
            CmdKind::Resume => {
                println!("→ Sending resume to agent {}", agent_id);
            }
        }

        Ok(())
    }
}

pub trait AgentInterface {
    fn send_command(&self, command: CmdKind, args: Vec<String>) -> impl std::future::Future<Output = AgentResult> + Send;
}

#[derive(Debug)]
pub enum AgentResult {
    Success,
    Retry,
    Failed(String),
}

pub async fn retry_with_backoff<F, Fut>(mut operation: F, max_attempts: u32) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<AgentResult>>,
{
    for attempt in 1..=max_attempts {
        match operation().await? {
            AgentResult::Success => return Ok(()),
            AgentResult::Failed(err) => return Err(anyhow::anyhow!("Agent failed: {}", err)),
            AgentResult::Retry => {
                if attempt < max_attempts {
                    let delay_ms = 100 * 2_u64.pow(attempt - 1);
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                } else {
                    return Err(anyhow::anyhow!("Max retry attempts reached"));
                }
            }
        }
    }
    Ok(())
}
