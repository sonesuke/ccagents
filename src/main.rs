use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use rule_agents::rule_engine::{decide_cmd, load_rules, RuleEngine};
use rule_agents::Manager;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to rules YAML file (used when no subcommand is provided)
    #[arg(short, long, global = true)]
    rules: Option<PathBuf>,

    /// Interval between simulated captures in seconds (used when no subcommand is provided)
    #[arg(short, long, default_value = "5", global = true)]
    interval: u64,
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
    /// Interval between simulated captures (seconds)
    #[arg(short, long, default_value = "5")]
    interval: u64,
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
        None => {
            // Default to daemon mode
            let rules_path = cli
                .rules
                .unwrap_or_else(|| PathBuf::from("examples/basic-rules.yaml"));
            let engine = RuleEngine::new(rules_path.to_str().unwrap())
                .await
                .context("Failed to create rule engine")?;

            println!("🚀 RuleAgents daemon started (default mode)");
            println!("📂 Watching rules file: {}", rules_path.display());
            println!("⏱️  Simulation interval: {}s", cli.interval);
            println!("📝 Edit the rules file to see hot-reload in action");
            println!("🛑 Press Ctrl+C to stop");

            let mut counter = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(cli.interval)).await;

                // Simulate different capture scenarios
                let test_captures = [
                    "issue 123",
                    "cancel operation",
                    "resume task",
                    "unknown scenario",
                ];

                let capture = test_captures[counter % test_captures.len()];
                let rules = engine.get_rules().await;
                let (command, cmd_args) = decide_cmd(capture, &rules);

                counter += 1;
                println!(
                    "[{}] Capture: \"{}\" → {:?} {:?}",
                    counter, capture, command, cmd_args
                );
            }
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
                let engine = RuleEngine::new(args.rules.to_str().unwrap())
                    .await
                    .context("Failed to create rule engine")?;

                println!("🚀 RuleAgents daemon started");
                println!("📂 Watching rules file: {}", args.rules.display());
                println!("⏱️  Simulation interval: {}s", args.interval);
                println!("📝 Edit the rules file to see hot-reload in action");
                println!("🛑 Press Ctrl+C to stop");

                let mut counter = 0;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(args.interval)).await;

                    // Simulate different capture scenarios
                    let test_captures = [
                        "issue 123",
                        "cancel operation",
                        "resume task",
                        "unknown scenario",
                    ];

                    let capture = test_captures[counter % test_captures.len()];
                    let rules = engine.get_rules().await;
                    let (command, cmd_args) = decide_cmd(capture, &rules);

                    counter += 1;
                    println!(
                        "[{}] Capture: \"{}\" → {:?} {:?}",
                        counter, capture, command, cmd_args
                    );
                }
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
        },
    }

    Ok(())
}
