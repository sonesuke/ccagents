use anyhow::{Context, Result};
use clap::Parser;
use rule_agents::rule_engine::load_rules;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the YAML rules file
    #[arg(short, long, default_value = "rules.yaml")]
    rules: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    // Load and compile rules from YAML file
    let rules = load_rules(&args.rules).context("Failed to load rules")?;

    println!("Loaded {} rules", rules.len());
    for rule in &rules {
        println!(
            "  Priority {}: {} -> {:?}",
            rule.priority,
            rule.regex.as_str(),
            rule.command
        );
    }

    // TODO: Integrate with rule engine in future phases
    tracing::info!(
        "Rules loaded successfully. Integration with rule engine will be implemented in Phase 2."
    );

    Ok(())
}
