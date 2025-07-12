use crate::agent::{Agent, AgentStatus};
use anyhow::Result;
use std::process::Command;

/// Get child processes of a given parent PID
pub fn get_child_processes(parent_pid: u32) -> Vec<u32> {
    let output = Command::new("pgrep")
        .arg("-P")
        .arg(parent_pid.to_string())
        .output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// Monitor command completion by checking child processes of the shell
pub async fn monitor_command_completion(agent: &Agent) {
    if let Ok(Some(shell_pid)) = agent.process.get_shell_pid().await {
        let child_pids = get_child_processes(shell_pid);
        let current_status = agent.get_status().await;

        match (!child_pids.is_empty(), current_status) {
            (true, AgentStatus::Idle) => {
                agent.set_status(AgentStatus::Active).await;
                tracing::debug!("🔄 Agent {} → Active", agent.get_id());
            }
            (false, AgentStatus::Active) => {
                agent.set_status(AgentStatus::Idle).await;
                tracing::debug!("✅ Agent {} → Idle", agent.get_id());
            }
            _ => {}
        }
    }
}

/// Start monitoring this agent's status (Active/Idle) based on child processes
pub async fn start_status_monitoring(agent: std::sync::Arc<Agent>) -> Result<()> {
    loop {
        monitor_command_completion(&agent).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
