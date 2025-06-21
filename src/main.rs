use anyhow::Result;
use clap::Parser;
use rule_agents::{RuleEngine, RuleFile};
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

    // Load rules from YAML file
    let rule_file = RuleFile::load(&args.rules)?;
    tracing::info!(
        "Loaded {} rules from {:?}",
        rule_file.rules.len(),
        args.rules
    );

    // Create rule engine
    let mut engine = RuleEngine::new();

    // Compile and register rules
    for rule in rule_file.rules {
        engine.register_rule(rule.into())?;
    }

    // Start the rule engine
    engine.start().await?;

    // Keep the engine running until Ctrl+C
    tracing::info!("Rule engine started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    // Gracefully shutdown
    engine.stop().await?;
    tracing::info!("Rule engine stopped.");

    Ok(())
}
