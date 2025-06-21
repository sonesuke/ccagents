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

            // Create a single agent for mock.sh testing
            ruler.create_agent("main").await?;

            println!("🎯 RuleAgents started");
            println!("📂 Rules file: {}", rules_path.display());
            println!("🌐 Terminal available at: http://localhost:9990");
            println!("💡 Type 'entry' in the terminal to start mock.sh");
            println!("🛑 Press Ctrl+C to stop");

            // Monitor terminal output and apply rules
            let agent = ruler.get_agent("main").await?;
            
            // Wait a moment for terminal to be ready, then start mock.sh
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("🚀 Starting mock.sh automatically...");
            if let Err(e) = agent.send_keys("entry\r").await {
                eprintln!("❌ Error starting mock.sh: {}", e);
            }
            
            loop {
                tokio::select! {
                    _ = signal::ctrl_c() => {
                        println!("\n🛑 Received Ctrl+C, shutting down...");
                        // Note: Child processes (HT) are automatically cleaned up by the OS
                        // when the parent process (rule-agents) terminates due to the 
                        // standard parent-child process relationship established by spawn()
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                        // Get terminal output
                        if let Ok(output) = agent.get_output().await {
                            if !output.trim().is_empty() {
                                // Check if output matches any rule
                                let action = ruler.decide_action_for_capture(&output).await;
                                
                                match action {
                                    ruler::rule_types::ActionType::SendKeys(keys) => {
                                        if !keys.is_empty() {
                                            println!("🤖 Matched: '{}' → Sending: {:?}", output.trim(), keys);
                                            
                                            // Send the keys to the terminal
                                            for key in keys {
                                                if key == "\\r" || key == "\r" {
                                                    if let Err(e) = agent.send_keys("\r").await {
                                                        eprintln!("❌ Error sending key: {}", e);
                                                    }
                                                } else {
                                                    if let Err(e) = agent.send_keys(&key).await {
                                                        eprintln!("❌ Error sending key: {}", e);
                                                    }
                                                }
                                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                            }
                                        }
                                    }
                                    ruler::rule_types::ActionType::Workflow(workflow_name, args) => {
                                        println!("🔄 Matched: '{}' → Workflow: {} {:?}", output.trim(), workflow_name, args);
                                    }
                                }
                            }
                        }
                    }
                }
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
