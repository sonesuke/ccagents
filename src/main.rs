use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use rule_agents::rule_engine::{decide_cmd, load_rules};
use rule_agents::Manager;
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
    /// Run daemon with hot-reload capability
    Daemon(DaemonArgs),
    /// Run manager simulation with agent scenarios
    Manager(ManagerArgs),
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

#[derive(Args, Debug)]
struct DaemonArgs {
    /// Path to rules YAML file
    #[arg(short, long, default_value = "examples/basic-rules.yaml")]
    rules: PathBuf,
}

#[derive(Args, Debug)]
struct ManagerArgs {
    /// Path to rules YAML file
    #[arg(short, long, default_value = "examples/basic-rules.yaml")]
    rules: PathBuf,
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
        Commands::Daemon(args) => {
            use rule_agents::rule_engine::RuleEngine;

            let mut rule_engine = RuleEngine::new(args.rules.to_str().unwrap()).await?;
            rule_engine.start().await?;

            println!("🚀 RuleAgents Daemon started");
            println!("📂 Rules file: {}", args.rules.display());
            println!("👀 Hot-reload enabled - rules will update automatically");
            println!("Press Ctrl+C to stop");

            // Keep daemon running
            tokio::signal::ctrl_c().await?;

            println!("🛑 Daemon shutting down...");
            rule_engine.stop().await?;
        }
        Commands::Manager(args) => {
            let manager = Manager::new(args.rules.to_str().unwrap()).await?;

            println!("🎯 RuleAgents Manager started");
            println!("📂 Rules file: {}", args.rules.display());
            println!("🤖 Simulating agent waiting scenarios...");

            // Simulate different agent waiting scenarios
            let scenarios = vec![
                ("agent-001", "issue 456 detected in process"),
                ("agent-002", "network connection failed"),
                ("agent-003", "cancel current operation"),
                ("agent-004", "resume normal operation"),
                ("agent-005", "unknown error occurred"),
            ];

            for (agent_id, capture) in scenarios {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                if let Err(e) = manager.handle_waiting_state(agent_id, capture).await {
                    eprintln!("❌ Error handling agent {}: {}", agent_id, e);
                }
            }

            println!("✅ Manager simulation complete");
        }
    }

    Ok(())
}
