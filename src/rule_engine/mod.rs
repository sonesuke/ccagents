pub mod compiled_rule;
pub mod rule_file;

pub use compiled_rule::CompiledRule;
pub use rule_file::{RuleFile, load_rules, CmdKind};

use anyhow::Result;

pub struct RuleEngine {
    rules: Vec<CompiledRule>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn register_rule(&mut self, rule: CompiledRule) -> Result<()> {
        self.rules.push(rule);
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting rule engine with {} rules", self.rules.len());
        // TODO: Implement rule engine start logic
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping rule engine");
        // TODO: Implement rule engine stop logic
        Ok(())
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
