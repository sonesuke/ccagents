mod agent;
mod cli;
mod monitor;
mod queue;
mod ruler;
mod web_server;
mod web_ui;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use monitor::{AgentSystem, TriggerSystem};
use queue::create_shared_manager;
use ruler::Ruler;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;

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

    // Run automation command (main mode)
    let rules_path = cli.config.unwrap_or_else(|| PathBuf::from("config.yaml"));
    run_automation_command(rules_path).await?;

    Ok(())
}

/// Run automation command (default mode when no subcommand is provided)
async fn run_automation_command(rules_path: PathBuf) -> Result<()> {
    // Create core components
    let queue_manager = create_shared_manager();
    let ruler = Arc::new(Ruler::new(rules_path.to_str().unwrap()).await?);

    let base_port = ruler.get_monitor_config().get_web_ui_port();

    println!("🎯 RuleAgents started");
    println!("📂 Config file: {}", rules_path.display());
    println!("🌐 Terminal available at: http://localhost:{}", base_port);
    println!("🛑 Press Ctrl+C to stop");

    // Create agent pool (includes web server management now)
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

    // 1. Start trigger system (startup + periodic)
    let on_start_entries = ruler.get_on_start_entries().await;
    let periodic_entries = ruler.get_periodic_entries().await;
    let trigger_system = TriggerSystem::new(
        on_start_entries,
        periodic_entries,
        Arc::clone(&agent_pool),
        queue_manager.clone(),
    );
    let trigger_handles = trigger_system.start_all_triggers().await?;

    // 2. Start agent system (monitoring)
    let agent_system = AgentSystem::new(Arc::clone(&ruler), Arc::clone(&agent_pool), queue_manager);
    let monitoring_handles = agent_system.start_monitoring().await?;

    // Wait for Ctrl+C signal
    signal::ctrl_c()
        .await
        .context("Failed to listen for ctrl_c")?;
    println!("\n🛑 Received Ctrl+C, shutting down...");

    // Shutdown all systems
    for handle in trigger_handles {
        handle.abort();
    }
    for handle in monitoring_handles {
        handle.abort();
    }

    println!("🧹 Shutting down...");

    // Force exit to ensure all threads terminate
    std::process::exit(0);
}
