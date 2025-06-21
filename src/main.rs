use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use rule_agents::rule_engine::{decide_cmd, load_rules};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Load and display rules
    Show(ShowArgs),
    /// Test rule matching against capture text
    Test(TestArgs),
}

#[derive(Args, Debug)]
struct ShowArgs {
    /// Path to rules YAML file
    #[arg(short, long, default_value = "rules.yaml")]
    rules: PathBuf,
}

#[derive(Args, Debug)]
struct TestArgs {
    /// Path to rules YAML file
    #[arg(short, long, default_value = "rules.yaml")]
    rules: PathBuf,
    /// Capture text to test against rules
    #[arg(short, long)]
    capture: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::Show(args) => {
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
        }
        Commands::Test(args) => {
            // Load rules and test against capture text
            let rules = load_rules(&args.rules).context("Failed to load rules")?;
            let (command, cmd_args) = decide_cmd(&args.capture, &rules);

            println!("Input: \"{}\"", args.capture);
            println!("Result: Command = {:?}, Args = {:?}", command, cmd_args);

            // Show which rule matched (if any)
            for rule in &rules {
                if rule.regex.is_match(&args.capture) {
                    println!(
                        "Matched rule: Priority {}, Pattern: \"{}\"",
                        rule.priority,
                        rule.regex.as_str()
                    );
                    break;
                }
            }
        }
    }

    Ok(())
}
