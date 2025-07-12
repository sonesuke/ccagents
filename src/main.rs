mod agent;
mod cli;
mod queue;
mod ruler;
mod web_server;
mod web_ui;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, execute_entry_action, execute_periodic_entry, process_pty_output};
use queue::create_shared_manager;
use ruler::Ruler;
use ruler::entry::TriggerType;
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
        tracing::Level::DEBUG // DEBUG level or higher when --debug
    } else {
        tracing::Level::WARN // WARN level or higher in normal operation (errors and warnings only)
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
        for (i, entry) in on_start_entries.iter().enumerate() {
            let agent = agent_pool.get_agent_by_index(i % agent_pool.size());
            println!(
                "🎯 Executing startup entry '{}' on agent {}",
                entry.name,
                agent.get_id()
            );
            execute_entry_action(&agent, entry, &queue_manager).await?;
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
                // Execute immediately on startup
                println!(
                    "⏰ Executing periodic entry immediately on startup: {}",
                    entry_clone.name
                );
                let agent = agent_pool_clone.get_agent_by_index(0);
                if let Err(e) =
                    execute_periodic_entry(&entry_clone, &queue_manager_clone, Some(&agent)).await
                {
                    eprintln!(
                        "❌ Error executing startup periodic entry '{}': {}",
                        entry_clone.name, e
                    );
                }

                // Continue with periodic execution
                let mut timer = interval(period);
                loop {
                    timer.tick().await;
                    println!("⏰ Executing periodic entry: {}", entry_clone.name);

                    // Check if there's data to process
                    match cli::has_data_to_process(&entry_clone).await {
                        Ok(true) => {
                            // Execute on available agent
                            let agent = agent_pool_clone.get_agent_by_index(0);
                            if let Err(e) = execute_periodic_entry(
                                &entry_clone,
                                &queue_manager_clone,
                                Some(&agent),
                            )
                            .await
                            {
                                eprintln!(
                                    "❌ Error executing periodic entry '{}': {}",
                                    entry_clone.name, e
                                );
                            }
                        }
                        Ok(false) => {
                            // No data to process
                            println!(
                                "ℹ️ No data to process for periodic entry: {}",
                                entry_clone.name
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "❌ Error checking data for periodic entry '{}': {}",
                                entry_clone.name, e
                            );
                        }
                    }
                }
            });
            periodic_handles.push(handle);
        }
    }

    // Start state-based monitoring loop
    let ruler = Arc::new(ruler); // Wrap ruler in Arc for sharing
    let agent_pool_for_monitoring = Arc::clone(&agent_pool);
    let ruler_for_monitoring = Arc::clone(&ruler);
    let queue_manager_for_monitoring = queue_manager.clone();

    let monitoring_handle = tokio::spawn(async move {
        // Create persistent receivers for each agent at startup
        let mut agent_receivers = Vec::new();
        for i in 0..agent_pool_for_monitoring.size() {
            let agent = agent_pool_for_monitoring.get_agent_by_index(i);
            match agent.get_pty_string_receiver().await {
                Ok(rx) => {
                    tracing::info!(
                        "✅ Agent {} persistent string receiver created",
                        agent.get_id()
                    );
                    agent_receivers.push(Some(rx));
                }
                Err(e) => {
                    tracing::error!(
                        "❌ Agent {} failed to create string receiver: {}",
                        agent.get_id(),
                        e
                    );
                    agent_receivers.push(None);
                }
            }
        }

        loop {
            // Monitor all agents for rule matching and status management
            for i in 0..agent_pool_for_monitoring.size() {
                let agent = agent_pool_for_monitoring.get_agent_by_index(i);

                // Always monitor command completion to auto-manage Active/Idle status
                agent.monitor_command_completion().await;

                let current_status = agent.get_status().await;

                // Use the persistent receiver for this agent
                if let Some(ref mut rx) = agent_receivers.get_mut(i).and_then(|r| r.as_mut()) {
                    // Check for new output (non-blocking)
                    let mut received_any = false;
                    while let Ok(pty_output) = rx.try_recv() {
                        received_any = true;
                        tracing::debug!(
                            "📝 Agent {} ({:?}) received PTY output: {} bytes: '{}'",
                            agent.get_id(),
                            current_status,
                            pty_output.len(),
                            pty_output.chars().take(50).collect::<String>()
                        );

                        // Process rules only for Active agents
                        // Rules should only be evaluated when agent is actively running commands
                        if current_status == agent::AgentStatus::Active {
                            tracing::debug!(
                                "🔍 Processing rules for agent {} ({:?})",
                                agent.get_id(),
                                current_status
                            );
                            if let Err(e) = process_pty_output(
                                &pty_output,
                                &agent,
                                &ruler_for_monitoring,
                                &queue_manager_for_monitoring,
                            )
                            .await
                            {
                                tracing::debug!("❌ Error processing PTY output: {}", e);
                            }
                        } else {
                            tracing::trace!(
                                "⏸️  Skipping rule processing for agent {} (status: {:?})",
                                agent.get_id(),
                                current_status
                            );
                        }
                    }

                    if received_any {
                        tracing::debug!(
                            "✅ Agent {} processed {} data chunks",
                            agent.get_id(),
                            "some"
                        );
                    }
                } else {
                    tracing::debug!("❌ Agent {} has no valid string receiver", agent.get_id());
                }
            }

            // Check for timeout rule triggers periodically
            // Only process timeout rules when agent is Active
            if agent_pool_for_monitoring.size() > 0 {
                let agent = agent_pool_for_monitoring.get_agent_by_index(0);
                let current_status = agent.get_status().await;

                if current_status == agent::AgentStatus::Active {
                    let timeout_actions = ruler_for_monitoring.check_timeout_rules().await;
                    for action in timeout_actions {
                        tracing::info!("⏰ Executing timeout rule action: {:?}", action);
                        if let Err(e) =
                            cli::execute_rule_action(&action, &agent, &queue_manager_for_monitoring)
                                .await
                        {
                            tracing::error!("❌ Error executing timeout rule action: {}", e);
                        }
                    }
                }
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });

    // Wait for Ctrl+C signal
    signal::ctrl_c()
        .await
        .context("Failed to listen for ctrl_c")?;
    println!("\n🛑 Received Ctrl+C, shutting down...");

    // Just abort all tasks - OS will clean up child processes
    for handle in periodic_handles {
        handle.abort();
    }
    for handle in web_server_handles {
        handle.abort();
    }
    monitoring_handle.abort();

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
    println!("  pool: {}", monitor_config.agents.pool);
    println!("  cols: {}", monitor_config.web_ui.cols);
    println!("  rows: {}", monitor_config.web_ui.rows);

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
            let rule_description = match &rule.rule_type {
                crate::ruler::rule::RuleType::Pattern(regex) => regex.as_str().to_string(),
                crate::ruler::rule::RuleType::DiffTimeout(duration) => {
                    format!("timeout:{:?}", duration)
                }
            };
            println!("  {}: {} -> {:?}", i + 1, rule_description, rule.action);
        }
    }

    Ok(())
}
