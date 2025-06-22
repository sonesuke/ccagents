mod agent;
mod cli;
mod queue;
mod ruler;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{
    execute_entry_action, execute_periodic_entry, process_terminal_output,
    resolve_entry_task_placeholders, Cli, Commands,
};
use queue::create_shared_manager;
use ruler::decision::decide_action;
use ruler::entry::TriggerType;
use ruler::Ruler;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio::time::interval;

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
            run_automation_command(rules_path).await?
        }
        Some(command) => match command {
            Commands::Show(args) => handle_show_command(&args).await?,
            Commands::Test(args) => handle_test_command(&args).await?,
        },
    }

    Ok(())
}

/// Run automation command (default mode when no subcommand is provided)
async fn run_automation_command(rules_path: PathBuf) -> Result<()> {
    // Create queue manager
    let queue_manager = create_shared_manager();

    // Create ruler with queue manager
    let ruler = Ruler::new(rules_path.to_str().unwrap()).await?;

    let base_port = ruler.get_monitor_config().base_port;

    println!("🎯 RuleAgents started");
    println!("📂 Config file: {}", rules_path.display());
    println!("🌐 Terminal available at: http://localhost:{}", base_port);
    println!("🛑 Press Ctrl+C to stop");

    // Create agent pool
    let monitor_config = ruler.get_monitor_config();
    let agent_pool = Arc::new(
        agent::AgentPool::new(
            monitor_config.agent_pool_size,
            monitor_config.base_port,
            false,
        )
        .await?,
    );

    // Wait a moment for terminal to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    println!("🚀 Ready to monitor terminal commands...");
    println!("💡 Agent pool size: {}", agent_pool.size());
    println!("💡 Open http://localhost:{} in your browser", base_port);

    // Execute on_start entries
    let on_start_entries = ruler.get_on_start_entries().await;
    if !on_start_entries.is_empty() {
        println!("🎬 Executing on_start entries...");
        for entry in on_start_entries {
            let agent = agent_pool.get_agent();
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
            let agent_pool_clone = Arc::clone(&agent_pool);

            let handle = tokio::spawn(async move {
                let mut timer = interval(period);
                loop {
                    timer.tick().await;
                    println!("⏰ Executing periodic entry: {}", entry_clone.name);
                    let agent = agent_pool_clone.get_agent();
                    if let Err(e) =
                        execute_periodic_entry(&entry_clone, &queue_manager_clone, Some(&agent))
                            .await
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
            let agent_pool_clone = Arc::clone(&agent_pool);
            let queue_manager_clone = queue_manager.clone();

            // Spawn task to listen for queue items
            let handle = tokio::spawn(async move {
                while let Some(task_item) = receiver.recv().await {
                    println!(
                        "🎯 Queue '{}' received item: '{}'",
                        queue_name_clone, task_item
                    );

                    // Resolve task placeholders in the entry
                    let resolved_entry = resolve_entry_task_placeholders(&entry_clone, &task_item);

                    // Execute the entry action with resolved placeholders
                    let agent = agent_pool_clone.get_agent();
                    if let Err(e) =
                        execute_entry_action(&agent, &resolved_entry, &queue_manager_clone).await
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

    let mut last_output: Option<String> = None;

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
                // Process terminal output and execute rules (use first agent for monitoring)
                let agent = agent_pool.get_agent();
                process_terminal_output(&agent, &ruler, &queue_manager, &mut last_output).await?;
            }
        }
    }

    Ok(())
}

/// Handle show command
async fn handle_show_command(args: &cli::ShowArgs) -> Result<()> {
    // Load and compile configuration from YAML file
    let (entries, rules, monitor_config) =
        ruler::config::load_config(&args.rules).context("Failed to load config")?;

    println!("Loaded {} entries and {} rules", entries.len(), rules.len());
    println!(
        "Monitor config: base_port = {}, agent_pool_size = {}",
        monitor_config.base_port, monitor_config.agent_pool_size
    );

    if !entries.is_empty() {
        println!("\nEntries:");
        for entry in &entries {
            println!(
                "  {}: {:?} -> {:?}",
                entry.name, entry.trigger, entry.action
            );
        }
    }

    if !rules.is_empty() {
        println!("\nRules:");
        for (i, rule) in rules.iter().enumerate() {
            println!("  {}: {} -> {:?}", i + 1, rule.regex.as_str(), rule.action);
        }
    }

    Ok(())
}

/// Handle test command
async fn handle_test_command(args: &cli::TestArgs) -> Result<()> {
    // Load rules and test against capture text
    let (_, rules, _) = ruler::config::load_config(&args.rules).context("Failed to load config")?;
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

    Ok(())
}
