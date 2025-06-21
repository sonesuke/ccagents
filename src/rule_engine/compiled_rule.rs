use super::rule_file::Rule;
use anyhow::Result;

#[derive(Debug)]
pub struct CompiledRule {
    pub name: String,
    pub description: String,
    // TODO: Add compiled rule structures
}

impl CompiledRule {
    pub fn compile(rule: Rule) -> Result<Self> {
        Ok(Self {
            name: rule.name,
            description: rule.description,
            // TODO: Implement rule compilation
        })
    }
}

impl From<Rule> for CompiledRule {
    fn from(rule: Rule) -> Self {
        Self::compile(rule).expect("Failed to compile rule")
    }
}
