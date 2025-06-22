mod agent;
mod ruler;
mod workflow;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use ruler::decision::decide_action;
use ruler::Ruler;
use std::path::PathBuf;
use tokio::signal;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to config YAML file
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
    /// Path to config YAML file
    #[arg(short, long, default_value = "config.yaml")]
    rules: PathBuf,
}

#[derive(Args, Debug)]
struct TestArgs {
    /// Path to config YAML file
    #[arg(short, long, default_value = "config.yaml")]
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
                .unwrap_or_else(|| PathBuf::from("config.yaml"));
            
            let mut ruler = Ruler::new(rules_path.to_str().unwrap()).await?;

            // Create a single agent for mock.sh testing
            ruler.create_agent("main").await?;

            println!("🎯 RuleAgents started");
            println!("📂 Config file: {}", rules_path.display());
            println!("🌐 Terminal available at: http://localhost:9990");
            println!("💡 Type 'entry' in the terminal to start mock.sh");
            println!("🛑 Press Ctrl+C to stop");

            // Monitor terminal output and apply rules
            let agent = ruler.get_agent("main").await?;
            
            // Wait a moment for terminal to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("🚀 Ready to monitor terminal commands...");
            println!("💡 Open http://localhost:9990 in your browser");
            
            // Execute on_start entries
            let on_start_entries = ruler.get_on_start_entries().await;
            if !on_start_entries.is_empty() {
                println!("🎬 Executing on_start entries...");
                for entry in on_start_entries {
                    match &entry.action {
                        ruler::types::ActionType::SendKeys(keys) => {
                            println!("🤖 Executing entry '{}' → Sending: {:?}", entry.name, keys);
                            
                            for key in keys {
                                if key == "\\r" || key == "\r" {
                                    if let Err(e) = agent.send_keys("\r").await {
                                        eprintln!("❌ Error sending key: {}", e);
                                    }
                                } else if let Err(e) = agent.send_keys(key).await {
                                    eprintln!("❌ Error sending key: {}", e);
                                }
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            }
                        }
                        ruler::types::ActionType::Workflow(workflow_name, args) => {
                            println!("🔄 Executing entry '{}' → Workflow: {} {:?}", entry.name, workflow_name, args);
                        }
                    }
                }
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
                                // === HT TERMINAL DIFFERENTIAL DETECTION STRATEGY ===
                                // HT Terminal sends the entire screen buffer (e.g., 120x40 characters) as a single
                                // continuous string without newline characters. The buffer is fixed-width with space
                                // padding, creating a format like:
                                // "command_line[padding spaces]output_line1[padding spaces]output_line2[padding]..."
                                //
                                // CHALLENGES:
                                // 1. No newline characters - entire buffer appears as one "line"
                                // 2. Space padding changes as new content appears
                                // 3. Same buffer length but different content triggers false "new line" detection
                                //
                                // SOLUTION: 
                                // - Compare buffers character-by-character
                                // - Extract only the newly added content at the END of the buffer
                                // - Ignore changes in the middle (cursor movements, overwrites)
                                //
                                // This treats the terminal as an append-only stream where only the suffix
                                // represents new terminal activity worthy of rule evaluation.
                                
                                let current_output = output.trim();
                                let mut new_content = String::new();
                                
                                if !last_output_lines.is_empty() {
                                    let previous_output = &last_output_lines[0];
                                    
                                    // Find the longest common prefix between previous and current output
                                    let common_prefix_len = previous_output.chars()
                                        .zip(current_output.chars())
                                        .take_while(|(a, b)| a == b)
                                        .count();
                                    
                                    // Extract the new content that was appended to the end
                                    // Handle Unicode character boundaries safely
                                    if current_output.len() > common_prefix_len {
                                        // Find a safe character boundary at or after common_prefix_len
                                        let safe_start = current_output.char_indices()
                                            .find(|(i, _)| *i >= common_prefix_len)
                                            .map(|(i, _)| i)
                                            .unwrap_or(current_output.len());
                                        
                                        if safe_start < current_output.len() {
                                            new_content = current_output[safe_start..].trim().to_string();
                                        }
                                    }
                                    
                                    println!("🔍 Buffer length: prev={}, curr={}", previous_output.len(), current_output.len());
                                    println!("🔍 Common prefix length: {}", common_prefix_len);
                                    // Filter out content that only contains ANSI escape sequences or whitespace
                                let meaningful_content = new_content.chars()
                                    .filter(|c| !c.is_whitespace() && *c != '\u{9b}' && !c.is_control())
                                    .count() > 0;
                                    
                                if !new_content.is_empty() {
                                    if meaningful_content {
                                        println!("📄 NEW content detected: {:?}", &new_content[..new_content.len().min(200)]);
                                    } else {
                                        println!("📄 Ignoring ANSI escape sequences");
                                    }
                                }
                                } else {
                                    // First time - entire output is "new"
                                    new_content = current_output.to_string();
                                    println!("📄 Initial buffer content: {:?}", &new_content[..new_content.len().min(200)]);
                                }
                                
                                // === SCRIPT STATE DETECTION ===
                                // Monitor script lifecycle by detecting key patterns in new content
                                if new_content.contains("=== Mock Test Script ===") {
                                    is_script_running = true;
                                    println!("🎬 Script started");
                                } else if is_script_running && (new_content.contains("MISSION COMPLETE") || new_content.contains("operation has been successfully completed")) {
                                    is_script_running = false;
                                    println!("💤 Script completed - Agent returned to idle state");
                                }
                                
                                // === RULE PROCESSING ON NEW CONTENT ===
                                // Apply rules only to the newly detected content
                                if !new_content.is_empty() && is_script_running && new_content.contains("Do you want to proceed") && !new_content.contains("MISSION COMPLETE") {
                                    println!("🎯 Found 'Do you want to proceed' in new content!");
                                    println!("🔍 New content: {:?}", &new_content[..new_content.len().min(300)]);
                                    println!("🔍 Script running: {}", is_script_running);
                                    println!("🔍 Contains MISSION COMPLETE: {}", new_content.contains("MISSION COMPLETE"));
                                    
                                    let action = ruler.decide_action_for_capture(&new_content).await;
                                    match action {
                                        ruler::types::ActionType::SendKeys(keys) => {
                                            if !keys.is_empty() {
                                                println!("🤖 EXECUTING RULE → Sending: {:?}", keys);
                                                println!("🕐 Timestamp: {}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
                                                
                                                // Send the keys to the terminal
                                                for (i, key) in keys.iter().enumerate() {
                                                    println!("  📤 Sending key {}: {:?}", i + 1, key);
                                                    if key == "\\r" || key == "\r" {
                                                        if let Err(e) = agent.send_keys("\r").await {
                                                            eprintln!("❌ Error sending key: {}", e);
                                                        }
                                                    } else if let Err(e) = agent.send_keys(key).await {
                                                        eprintln!("❌ Error sending key: {}", e);
                                                    }
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                                }
                                                
                                                println!("✅ Rule execution completed, waiting 1000ms");
                                                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                            }
                                        }
                                        ruler::types::ActionType::Workflow(workflow_name, args) => {
                                            println!("🔄 Matched workflow: {} {:?}", workflow_name, args);
                                        }
                                    }
                                }
                                
                                // Check for idle state in new content
                                if !new_content.is_empty() && !is_script_running && new_content.contains("sonesuke@Air") && new_content.contains("%") {
                                    println!("⏸️ Idle state detected in new content");
                                }
                                
                                // Update the stored output for next comparison
                                // Store as single string since HT terminal sends everything as one buffer
                                last_output_lines = vec![current_output.to_string()];
                            }
                        }
                    }
                }
            }
        }
        Some(command) => match command {
            Commands::Show(args) => {
                // Load and compile configuration from YAML file
                let (entries, rules) = ruler::config::load_config(&args.rules).context("Failed to load config")?;

                println!("Loaded {} entries and {} rules", entries.len(), rules.len());
                
                if !entries.is_empty() {
                    println!("\nEntries:");
                    for entry in &entries {
                        println!("  {}: {:?} -> {:?}", entry.name, entry.trigger, entry.action);
                    }
                }
                
                if !rules.is_empty() {
                    println!("\nRules:");
                    for (i, rule) in rules.iter().enumerate() {
                        println!("  {}: {} -> {:?}", i + 1, rule.regex.as_str(), rule.action);
                    }
                }
            }
            Commands::Test(args) => {
                // Load rules and test against capture text
                let (_, rules) = ruler::config::load_config(&args.rules).context("Failed to load config")?;
                let action = decide_action(&args.capture, &rules);

                println!("Input: \"{}\"", args.capture);
                println!("Result: Action = {:?}", action);

                // Show which rule matched (if any)
                for (i, rule) in rules.iter().enumerate() {
                    if rule.regex.is_match(&args.capture) {
                        println!("Matched rule: #{}, Pattern: \"{}\"", i + 1, rule.regex.as_str());
                        break;
                    }
                }
            }
        },
    }

    Ok(())
}
