mod agent;
mod ruler;
mod workflow;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use ruler::decision::decide_action;
use ruler::rule_loader::load_rules;
use ruler::Ruler;
use std::path::PathBuf;
use tokio::signal;
use workflow::hot_reload::HotReloader;
use workflow::Workflow;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to rules YAML file
    #[arg(short, long, global = true)]
    rules: Option<PathBuf>,
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


async fn setup_signal_handler() {
    let sigint = signal::ctrl_c();

    tokio::spawn(async move {
        if let Ok(()) = sigint.await {
            println!("\nReceived Ctrl+C, shutting down...");
            std::process::exit(0);
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        None => {
            // When no subcommand is provided, run manager mode (default)
            let rules_path = cli
                .rules
                .unwrap_or_else(|| PathBuf::from("examples/basic-rules.yaml"));
            
            let mut ruler = Ruler::new(rules_path.to_str().unwrap()).await?;
            let workflow = Workflow::new(false, Some(rules_path.to_str().unwrap())).await?;

            // Create agents for simulation
            ruler.create_agent("agent-001").await?;
            ruler.create_agent("agent-002").await?;
            ruler.create_agent("agent-003").await?;
            ruler.create_agent("agent-004").await?;

            // Set up Ctrl+C signal handler
            setup_signal_handler().await;

            println!("🎯 RuleAgents started");
            println!("📂 Rules file: {}", rules_path.display());
            println!("🤖 Simulating agent waiting scenarios...");
            println!("🛑 Press Ctrl+C to stop");

            // Simulate different agent waiting scenarios
            let scenarios = vec![
                ("agent-001", "issue 456 detected in process"),
                ("agent-002", "network connection failed"),
                ("agent-003", "resume normal operation"),
                ("agent-004", "unknown error occurred"),
            ];

            for (agent_id, capture) in scenarios {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                let agent = ruler.get_agent(agent_id).await?;
                let action = ruler.decide_action_for_capture(capture).await;
                if let Err(e) = workflow.handle_waiting_state(agent, capture, action).await {
                    eprintln!("❌ Error handling agent {}: {}", agent_id, e);
                }
            }

            println!("✅ Simulation complete");
        }
        Some(command) => match command {
            Commands::Show(args) => {
                // Load and compile rules from YAML file
                let rules = load_rules(&args.rules).context("Failed to load rules")?;

                println!("Loaded {} rules", rules.len());
                for rule in &rules {
                    println!(
                        "  Priority {}: {} -> {:?}",
                        rule.priority,
                        rule.regex.as_str(),
                        rule.action
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
                let action = decide_action(&args.capture, &rules);

                println!("Input: \"{}\"", args.capture);
                println!("Result: Action = {:?}", action);

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
        },
    }

    Ok(())
}
