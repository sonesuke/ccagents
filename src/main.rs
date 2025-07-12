mod agent;
mod cli;
mod config;
mod monitor;
mod trigger;
mod web_server;
mod web_ui;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use config::Config;
use monitor::Agents;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use trigger::Triggers;

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
    let config = Arc::new(Config::new(rules_path.to_str().unwrap()).await?);

    let base_port = config.get_monitor_config().get_web_ui_port();

    println!("🎯 RuleAgents started");
    println!("📂 Config file: {}", rules_path.display());
    println!("🌐 Terminal available at: http://localhost:{}", base_port);
    println!("🛑 Press Ctrl+C to stop");

    // Create agent pool (includes web server management now)
    let agent_pool = Arc::new(agent::AgentPool::new(config.get_monitor_config()).await?);

    // 1. Start triggers (startup + periodic)
    let triggers = Triggers::new(config.get_trigger_config(), Arc::clone(&agent_pool));
    let trigger_handles = triggers.start_all().await?;

    // 2. Start agents (monitoring)
    let agents = Agents::new(config.get_rule_config(), Arc::clone(&agent_pool));
    let agent_handles = agents.start_monitoring().await?;

    // Wait for Ctrl+C signal
    signal::ctrl_c()
        .await
        .context("Failed to listen for ctrl_c")?;
    println!("\n🛑 Received Ctrl+C, shutting down...");

    // Shutdown all systems
    for handle in trigger_handles {
        handle.abort();
    }
    for handle in agent_handles {
        handle.abort();
    }

    println!("🧹 Shutting down...");

    // Force exit to ensure all threads terminate
    std::process::exit(0);
}
