mod agent;
mod cli;
mod queue;
mod ruler;
mod web_server;
mod web_ui;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{
    execute_entry_action, execute_periodic_entry, process_direct_output, process_pty_output,
    resolve_entry_task_placeholders, Cli, Commands,
};
use queue::create_shared_manager;
use ruler::entry::TriggerType;
use ruler::Ruler;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio::time::interval;
use web_server::WebServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments first to get debug flag
    let cli = Cli::parse();

    // Initialize logging based on debug flag
    let level = if cli.debug {
        tracing::Level::DEBUG // --debug時はDEBUGレベル以上
    } else {
        tracing::Level::WARN // 通常時はWARNレベル以上（エラーと警告のみ）
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    match cli.command {
        None => {
            // When no subcommand is provided, run manager mode (default)
            let rules_path = cli.config.unwrap_or_else(|| PathBuf::from("config.yaml"));
            run_automation_command(rules_path).await?
        }
        Some(command) => match command {
            Commands::Show(args) => handle_show_command(&args).await?,
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

    let base_port = ruler.get_monitor_config().get_web_ui_port();

    println!("🎯 RuleAgents started");
    println!("📂 Config file: {}", rules_path.display());
    println!("🌐 Terminal available at: http://localhost:{}", base_port);
    println!("🛑 Press Ctrl+C to stop");

    // Create agent pool
    let monitor_config = ruler.get_monitor_config();
    let agent_pool = Arc::new(
        agent::AgentPool::new(
            monitor_config.get_agent_pool_size(),
            monitor_config.get_web_ui_port(),
            false,
            monitor_config,
        )
        .await?,
    );

    // Start web servers for each agent (if enabled)
    let mut web_server_handles = Vec::new();
    if monitor_config.web_ui.enabled {
        for i in 0..monitor_config.get_agent_pool_size() {
            let port = monitor_config.get_web_ui_port() + i as u16;
            let agent = agent_pool.get_agent_by_index(i);
            let web_server = WebServer::new(port, monitor_config.web_ui.host.clone(), agent);

            let handle = tokio::spawn(async move {
                if let Err(e) = web_server.start().await {
                    eprintln!("❌ Web server failed on port {}: {}", port, e);
                }
            });
            web_server_handles.push(handle);
        }
    }

    // Wait a moment for terminal to be ready
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    println!("🚀 Ready to monitor terminal commands...");
    println!("💡 Agent pool size: {}", agent_pool.size());

    // Show all web UI URLs (if enabled)
    if monitor_config.web_ui.enabled {
        for i in 0..monitor_config.get_agent_pool_size() {
            let port = monitor_config.get_web_ui_port() + i as u16;
            println!(
                "💡 Agent {} web UI: http://{}:{}",
                i + 1,
                monitor_config.web_ui.host,
                port
            );
        }
    } else {
        println!("💡 Web UI disabled in configuration");
    }

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
    tracing::debug!("📡 Setting up {} queue listeners...", enqueue_entries.len());
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

    // Start PTY output monitoring tasks for each agent
    let ruler = Arc::new(ruler); // Wrap ruler in Arc for sharing
    let mut pty_monitor_handles = Vec::new();
    for i in 0..agent_pool.size() {
        let agent = agent_pool.get_agent_by_index(i);
        let ruler_clone = Arc::clone(&ruler);
        let queue_manager_clone = queue_manager.clone();

        let handle = tokio::spawn(async move {
            tracing::debug!("🔍 Starting PTY monitor task for agent {}", i);

            // Get PTY output receiver
            if let Ok(mut rx) = agent.get_pty_output_receiver().await {
                tracing::debug!("✅ Got PTY receiver for agent {}", i);

                // Continuously monitor PTY output
                while let Ok(pty_output) = rx.recv().await {
                    if let Err(e) =
                        process_pty_output(&pty_output, &agent, &ruler_clone, &queue_manager_clone)
                            .await
                    {
                        tracing::debug!("❌ Error processing PTY output: {}", e);
                    }
                }

                tracing::debug!("❌ PTY monitor task ended for agent {}", i);
            } else {
                tracing::debug!("❌ Failed to get PTY receiver for agent {}", i);
            }
        });
        pty_monitor_handles.push(handle);
    }

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("\n🛑 Received Ctrl+C, shutting down...");
        }
        _ = async {
            let mut interval_timer = tokio::time::interval(tokio::time::Duration::from_millis(500));
            loop {
                interval_timer.tick().await;
                // Process only direct command output (no terminal diff detection)
                let agent = agent_pool.get_agent();
                if let Err(e) = process_direct_output(&agent, &ruler, &queue_manager).await {
                    eprintln!("❌ Error processing output: {}", e);
                }
            }
        } => {
            // This branch should never be reached since the loop is infinite
        }
    }

    // Just abort all tasks - OS will clean up child processes
    for handle in periodic_handles {
        handle.abort();
    }
    for handle in queue_handles {
        handle.abort();
    }
    for handle in web_server_handles {
        handle.abort();
    }
    for handle in pty_monitor_handles {
        handle.abort();
    }

    println!("🧹 Shutting down...");

    // Force exit to ensure all threads terminate
    std::process::exit(0);
}

/// Handle show command
async fn handle_show_command(args: &cli::ShowArgs) -> Result<()> {
    // Load and compile configuration from YAML file
    let (entries, rules, monitor_config) =
        ruler::config::load_config(&args.config).context("Failed to load config")?;

    println!("Loaded {} entries and {} rules", entries.len(), rules.len());
    println!("\nWeb UI config:");
    println!("  enabled: {}", monitor_config.web_ui.enabled);
    println!("  host: {}", monitor_config.web_ui.host);
    println!("  base_port: {}", monitor_config.web_ui.base_port);
    println!("\nAgents config:");
    println!("  concurrency: {}", monitor_config.agents.concurrency);
    println!("  cols: {}", monitor_config.agents.cols);
    println!("  rows: {}", monitor_config.agents.rows);

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
