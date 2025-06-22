mod agent;
mod queue;
mod ruler;
mod workflow;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use queue::{create_shared_manager, QueueExecutor};
use ruler::decision::decide_action;
use ruler::entry::TriggerType;
use ruler::rule::resolve_task_placeholder_in_vec;
use ruler::Ruler;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio::time::interval;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to config YAML file
    #[arg(short, long, global = true)]
    rules: Option<PathBuf>,

    /// Enable debug logging for internal details
    #[arg(short, long, global = true)]
    debug: bool,
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
    // Parse command line arguments first to get debug flag
    let cli = Cli::parse();

    // Initialize logging based on debug flag
    if cli.debug {
        tracing_subscriber::fmt::init();
    } else {
        // Only show error logs when debug is disabled
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .init();
    }

    match cli.command {
        None => {
            // When no subcommand is provided, run manager mode (default)
            let rules_path = cli.rules.unwrap_or_else(|| PathBuf::from("config.yaml"));

            // Create queue manager
            let queue_manager = create_shared_manager();

            // Create ruler with queue manager
            let ruler = Ruler::with_queue_manager(
                rules_path.to_str().unwrap(),
                Some(queue_manager.clone()),
            )
            .await?;

            println!("🎯 RuleAgents started");
            println!("📂 Config file: {}", rules_path.display());
            println!("🌐 Terminal available at: http://localhost:9990");
            println!("💡 Type 'entry' in the terminal to start mock.sh");
            println!("🛑 Press Ctrl+C to stop");

            // Create agent directly
            let agent = Arc::new(agent::Agent::new("main".to_string(), false, 9990).await?);

            // Wait a moment for terminal to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("🚀 Ready to monitor terminal commands...");
            println!("💡 Open http://localhost:9990 in your browser");

            // Execute on_start entries
            let on_start_entries = ruler.get_on_start_entries().await;
            if !on_start_entries.is_empty() {
                println!("🎬 Executing on_start entries...");
                for entry in on_start_entries {
                    execute_entry_action(&agent, &entry, &queue_manager).await?;
                }
            }

            // Setup periodic timers
            let periodic_entries = ruler.get_periodic_entries().await;
            let mut periodic_handles = Vec::new();
            for entry in periodic_entries {
                if let TriggerType::Periodic { interval: period } = entry.trigger {
                    let entry_clone = entry.clone();
                    let queue_manager_clone = queue_manager.clone();

                    let handle = tokio::spawn(async move {
                        let mut timer = interval(period);
                        loop {
                            timer.tick().await;
                            println!("⏰ Executing periodic entry: {}", entry_clone.name);
                            // For periodic execution, we don't have an agent context yet
                            // This is for background tasks that don't need terminal interaction
                            if let Err(e) =
                                execute_periodic_entry(&entry_clone, &queue_manager_clone).await
                            {
                                eprintln!(
                                    "❌ Error executing periodic entry '{}': {}",
                                    entry_clone.name, e
                                );
                            }
                        }
                    });
                    periodic_handles.push(handle);
                }
            }

            // Setup queue listeners for enqueue entries
            let enqueue_entries = ruler.get_enqueue_entries().await;
            println!("📡 Setting up {} queue listeners...", enqueue_entries.len());
            let mut queue_handles = Vec::new();
            for entry in enqueue_entries {
                if let TriggerType::Enqueue { queue_name } = &entry.trigger {
                    let queue_name_clone = queue_name.clone();
                    println!("📡 Listening to queue: {}", queue_name);

                    // Subscribe to queue and get receiver
                    let mut receiver = {
                        let mut manager = queue_manager.write().await;
                        manager.subscribe(queue_name)
                    };

                    // Clone necessary data for the async task
                    let entry_clone = entry;
                    let agent_clone = Arc::clone(&agent);
                    let queue_manager_clone = queue_manager.clone();

                    // Spawn task to listen for queue items
                    let handle = tokio::spawn(async move {
                        while let Some(task_item) = receiver.recv().await {
                            println!(
                                "🎯 Queue '{}' received item: '{}'",
                                queue_name_clone, task_item
                            );

                            // Resolve task placeholders in the entry
                            let resolved_entry =
                                resolve_entry_task_placeholders(&entry_clone, &task_item);

                            // Execute the entry action with resolved placeholders
                            if let Err(e) = execute_entry_action(
                                &agent_clone,
                                &resolved_entry,
                                &queue_manager_clone,
                            )
                            .await
                            {
                                println!(
                                    "❌ Error executing queue entry '{}': {}",
                                    resolved_entry.name, e
                                );
                            }
                        }
                    });
                    queue_handles.push(handle);
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

                                    // Debug info (only shown with --debug flag)
                                    info!("Buffer length: prev={}, curr={}", previous_output.len(), current_output.len());
                                    info!("Common prefix length: {}", common_prefix_len);
                                if !new_content.is_empty() {
                                    // Always clean the content first
                                    let clean_content = agent::ht_process::HtProcess::clean_terminal_output(&new_content);

                                    // Check if cleaned content has meaningful text
                                    if !clean_content.trim().is_empty() {
                                        println!("📄 NEW content detected: {:?}", &clean_content[..clean_content.len().min(200)]);
                                    } else {
                                        println!("📄 Ignoring ANSI escape sequences");
                                    }
                                }
                                } else {
                                    // First time - entire output is "new"
                                    new_content = current_output.to_string();
                                    let clean_content = agent::ht_process::HtProcess::clean_terminal_output(&new_content);
                                    println!("📄 Initial buffer content: {:?}", &clean_content[..clean_content.len().min(200)]);
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
                                        ruler::types::ActionType::Enqueue { queue, command } => {
                                            println!("📦 Matched enqueue to '{}': {}", queue, command);
                                            let executor = QueueExecutor::new(queue_manager.clone());
                                            match executor.execute_and_enqueue(&queue, &command).await {
                                                Ok(count) => {
                                                    println!("✅ Enqueued {} items to queue '{}'", count, queue);
                                                }
                                                Err(e) => {
                                                    eprintln!("❌ Error executing enqueue action: {}", e);
                                                }
                                            }
                                        }
                                        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
                                            println!("📦 Matched enqueue_dedupe to '{}': {}", queue, command);
                                            let executor = QueueExecutor::new(queue_manager.clone());
                                            match executor.execute_and_enqueue_dedupe(&queue, &command).await {
                                                Ok(count) => {
                                                    println!("✅ Enqueued {} new items to dedupe queue '{}'", count, queue);
                                                }
                                                Err(e) => {
                                                    eprintln!("❌ Error executing enqueue_dedupe action: {}", e);
                                                }
                                            }
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
                let (entries, rules, monitor_config) =
                    ruler::config::load_config(&args.rules).context("Failed to load config")?;

                println!("Loaded {} entries and {} rules", entries.len(), rules.len());
                println!("Monitor config: base_port = {}", monitor_config.base_port);

                if !entries.is_empty() {
                    println!("\nEntries:");
                    for entry in &entries {
                        println!(
                            "  {}: {:?} -> {:?} (concurrency: {})",
                            entry.name, entry.trigger, entry.action, entry.concurrency
                        );
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
                let (_, rules, _) =
                    ruler::config::load_config(&args.rules).context("Failed to load config")?;
                let action = decide_action(&args.capture, &rules);

                println!("Input: \"{}\"", args.capture);
                println!("Result: Action = {:?}", action);

                // Show which rule matched (if any)
                for (i, rule) in rules.iter().enumerate() {
                    if rule.regex.is_match(&args.capture) {
                        println!(
                            "Matched rule: #{}, Pattern: \"{}\"",
                            i + 1,
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

/// Execute a periodic entry action (without agent context)
async fn execute_periodic_entry(
    entry: &ruler::entry::CompiledEntry,
    queue_manager: &queue::SharedQueueManager,
) -> Result<()> {
    match &entry.action {
        ruler::types::ActionType::SendKeys(_keys) => {
            println!(
                "⚠️ Periodic entry '{}' has SendKeys action - skipping (no agent context)",
                entry.name
            );
        }
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            println!(
                "🔄 Executing periodic entry '{}' → Workflow: {} {:?}",
                entry.name, workflow_name, args
            );
            // TODO: Implement workflow execution
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            println!(
                "📦 Executing periodic entry '{}' → Enqueue to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue(queue, command).await?;
            println!("✅ Enqueued {} items to queue '{}'", count, queue);
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            println!(
                "📦 Executing periodic entry '{}' → EnqueueDedupe to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue_dedupe(queue, command).await?;
            println!(
                "✅ Enqueued {} new items to dedupe queue '{}'",
                count, queue
            );
        }
    }
    Ok(())
}

/// Execute an entry action using the appropriate mechanism
async fn execute_entry_action(
    agent: &agent::Agent,
    entry: &ruler::entry::CompiledEntry,
    queue_manager: &queue::SharedQueueManager,
) -> Result<()> {
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
            println!(
                "🔄 Executing entry '{}' → Workflow: {} {:?}",
                entry.name, workflow_name, args
            );
            // TODO: Implement workflow execution
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            println!(
                "📦 Executing entry '{}' → Enqueue to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue(queue, command).await?;
            println!("✅ Enqueued {} items to queue '{}'", count, queue);
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            println!(
                "📦 Executing entry '{}' → EnqueueDedupe to '{}': {}",
                entry.name, queue, command
            );
            let executor = QueueExecutor::new(queue_manager.clone());
            let count = executor.execute_and_enqueue_dedupe(queue, command).await?;
            println!(
                "✅ Enqueued {} new items to dedupe queue '{}'",
                count, queue
            );
        }
    }
    Ok(())
}

/// Resolve <task> placeholders in entry action with actual task value
fn resolve_entry_task_placeholders(
    entry: &ruler::entry::CompiledEntry,
    task_value: &str,
) -> ruler::entry::CompiledEntry {
    let resolved_action = match &entry.action {
        ruler::types::ActionType::SendKeys(keys) => {
            ruler::types::ActionType::SendKeys(resolve_task_placeholder_in_vec(keys, task_value))
        }
        ruler::types::ActionType::Workflow(workflow_name, args) => {
            let resolved_workflow =
                ruler::rule::resolve_task_placeholder(workflow_name, task_value);
            let resolved_args = resolve_task_placeholder_in_vec(args, task_value);
            ruler::types::ActionType::Workflow(resolved_workflow, resolved_args)
        }
        ruler::types::ActionType::Enqueue { queue, command } => {
            let resolved_queue = ruler::rule::resolve_task_placeholder(queue, task_value);
            let resolved_command = ruler::rule::resolve_task_placeholder(command, task_value);
            ruler::types::ActionType::Enqueue {
                queue: resolved_queue,
                command: resolved_command,
            }
        }
        ruler::types::ActionType::EnqueueDedupe { queue, command } => {
            let resolved_queue = ruler::rule::resolve_task_placeholder(queue, task_value);
            let resolved_command = ruler::rule::resolve_task_placeholder(command, task_value);
            ruler::types::ActionType::EnqueueDedupe {
                queue: resolved_queue,
                command: resolved_command,
            }
        }
    };

    ruler::entry::CompiledEntry {
        name: entry.name.clone(),
        trigger: entry.trigger.clone(),
        action: resolved_action,
        concurrency: entry.concurrency,
    }
}
