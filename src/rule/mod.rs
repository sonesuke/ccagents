pub mod diff_timeout;
pub mod when;

// Re-export for convenience
pub use diff_timeout::DiffTimeout;
pub use when::When;

use crate::agent::Agent;
use crate::config::helper::ActionType;
use anyhow::Result;
use tokio::sync::broadcast;
use tokio::time::Duration as TokioDuration;

/// Common trait for all rule processors
pub trait RuleProcessor {
    async fn start_monitoring(&self, receiver: broadcast::Receiver<String>) -> Result<()>;
}

/// Execute an action for rules with delay between keys
pub async fn execute_rule_action(action: &ActionType, agent: &Agent, context: &str) -> Result<()> {
    let ActionType::SendKeys(keys) = action;

    if keys.is_empty() {
        tracing::debug!("{}: No keys to send", context);
        return Ok(());
    }

    tracing::info!("{}: Sending {} keys", context, keys.len());
    tracing::debug!("{}: Keys: {:?}", context, keys);

    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(TokioDuration::from_millis(100)).await;
        }
        agent.send_keys(key).await?;
    }

    Ok(())
}
