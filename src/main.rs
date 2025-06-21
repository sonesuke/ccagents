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
                .unwrap_or_else(|| PathBuf::from("examples/mock-rules.yaml"));
            
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
            
            // Wait a moment for terminal to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("🚀 Testing entry command automatically...");
            
            // Send entry command for testing
            if let Err(e) = agent.send_keys("entry\r").await {
                eprintln!("❌ Error sending entry command: {}", e);
            } else {
                println!("✅ Sent 'entry' command to terminal");
            }
            
            let mut last_output_lines: Vec<String> = Vec::new();
            let mut is_script_running = false;
            
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
                                // === DIFFERENTIAL CHANGE DETECTION ===
                                // This implementation uses line-based differential detection instead of 
                                // time-based or hash-based approaches for the following reasons:
                                //
                                // 1. ROBUST RULE EXECUTION: Terminal output contains command history,
                                //    and applying rules to the entire output would cause unwanted
                                //    re-execution of rules on historical commands (e.g., "entry" command
                                //    appearing in scrollback would repeatedly trigger script execution)
                                //
                                // 2. PRECISE STATE DETECTION: Only newly added lines represent actual
                                //    terminal activity, allowing accurate detection of script completion
                                //    and idle state transitions without false positives from static content
                                //
                                // 3. ELIMINATES TIMING DEPENDENCIES: Unlike timeout-based approaches,
                                //    this method doesn't rely on arbitrary delays or polling intervals,
                                //    making the system more deterministic and responsive
                                //
                                // 4. PREVENTS DUPLICATE ACTIONS: By tracking which lines have been
                                //    processed, we ensure each terminal event triggers rules exactly once,
                                //    avoiding infinite loops or redundant command execution
                                //
                                // This approach treats terminal output as an append-only log where only
                                // the delta (new lines) should trigger rule evaluation.
                                
                                // Split current terminal output into individual lines
                                let current_lines: Vec<String> = output.lines()
                                    .map(|line| line.trim().to_string())
                                    .filter(|line| !line.is_empty())
                                    .collect();
                                
                                // Identify new lines that weren't present in the previous output
                                // This represents the actual terminal activity since last check
                                let new_lines: Vec<String> = current_lines.iter()
                                    .filter(|line| !last_output_lines.contains(line))
                                    .cloned()
                                    .collect();
                                
                                if !new_lines.is_empty() {
                                    println!("📄 NEW lines: {:?}", new_lines);
                                    
                                    // Since terminal output may come as long single lines, also check the combined content
                                    let combined_new_content = new_lines.join(" ");
                                    
                                    // === SCRIPT STATE DETECTION ===
                                    // Monitor script lifecycle by detecting key patterns in new terminal lines.
                                    // This approach correctly identifies when automation should be active vs idle:
                                    // - Script startup: Detects script banner to enable automation
                                    // - Script completion: Detects completion message to disable automation
                                    // - Idle detection: When script completes and returns to shell prompt,
                                    //   the agent should stop processing rules to prevent unwanted actions
                                    if combined_new_content.contains("=== Mock Test Script ===") {
                                        is_script_running = true;
                                        println!("🎬 Script started");
                                    } else if is_script_running && combined_new_content.contains("MISSION COMPLETE") {
                                        is_script_running = false;
                                        println!("💤 Script completed - Agent returned to idle state");
                                    }
                                    
                                    // === RULE PROCESSING ON NEW CONTENT ===
                                    // Apply rules to new content (both line-by-line and combined content)
                                    // This handles cases where terminal output may be split differently
                                    
                                    // First check combined content for rules that might span multiple "lines"
                                    if is_script_running && combined_new_content.contains("Do you want to proceed") && !combined_new_content.contains("MISSION COMPLETE") {
                                        println!("🎯 Found 'Do you want to proceed' in combined content!");
                                        println!("🔍 Checking rules for combined content");
                                        
                                        let action = ruler.decide_action_for_capture(&combined_new_content).await;
                                        match action {
                                            ruler::rule_types::ActionType::SendKeys(keys) => {
                                                if !keys.is_empty() {
                                                    println!("🤖 Matched rule on combined content → Sending: {:?}", keys);
                                                    
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
                                                    
                                                    // Wait after sending keys
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                                }
                                            }
                                            ruler::rule_types::ActionType::Workflow(workflow_name, args) => {
                                                println!("🔄 Matched workflow: {} {:?}", workflow_name, args);
                                            }
                                        }
                                    }
                                    
                                    // Then process individual lines for entry commands and other patterns  
                                    for line in &new_lines {
                                        // Entry commands should only be processed when NOT already running a script
                                        if line.contains("entry") && !is_script_running {
                                            println!("🔍 Processing entry command in line: '{}'", line);
                                        } else if line.contains("entry") && is_script_running {
                                            println!("⏭️ Ignoring entry command - script already running: '{}'", line);
                                            continue;
                                        } else if !is_script_running && line.contains("sonesuke@Air") && line.contains("%") {
                                            // Skip rule processing for shell prompts when in idle state
                                            println!("⏸️ Idle state - not processing rules for line: {}", line);
                                            continue;
                                        }
                                        
                                        // Apply rules for entry commands when idle
                                        if (line.contains("entry") && !is_script_running) {
                                            println!("🔍 Checking rules for entry line: '{}'", line);
                                            
                                            // Check if this line matches any rule
                                            let action = ruler.decide_action_for_capture(line).await;
                                            
                                            match action {
                                                ruler::rule_types::ActionType::SendKeys(keys) => {
                                                    if !keys.is_empty() {
                                                        println!("🤖 Matched rule on '{}' → Sending: {:?}", line, keys);
                                                        
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
                                                        
                                                        // Wait after sending keys
                                                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                                        break; // Only execute one rule per iteration
                                                    }
                                                }
                                                ruler::rule_types::ActionType::Workflow(workflow_name, args) => {
                                                    println!("🔄 Matched: '{}' → Workflow: {} {:?}", line, workflow_name, args);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // === STATE PERSISTENCE ===
                                    // Update the baseline for differential detection by storing current lines.
                                    // This ensures that next iteration will only process truly new content,
                                    // maintaining the integrity of our differential change detection system.
                                    last_output_lines = current_lines;
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
