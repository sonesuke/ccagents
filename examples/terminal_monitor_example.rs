use rule_agents::ht_process::HtProcessConfig;
use rule_agents::{AgentState, HtClient, HtProcess, MonitorConfig, TerminalOutputMonitor};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting terminal output monitoring example");

    // Create HT process configuration
    let ht_config = HtProcessConfig {
        ht_binary_path: "ht".to_string(),
        shell_command: Some("bash".to_string()),
        restart_attempts: 3,
        restart_delay_ms: 1000,
    };

    // Create and start HT process
    let ht_process = HtProcess::new(ht_config);
    let ht_client = Arc::new(HtClient::new(ht_process));

    info!("Starting HT client...");
    ht_client.start().await?;

    info!("Note: HT terminal web interface is available at http://localhost:9999");

    // Create terminal output monitor with custom configuration
    let monitor_config = MonitorConfig {
        change_detection_interval: Duration::from_millis(250), // Check every 250ms
        wait_timeout: Duration::from_secs(1),                  // Wait state after 1s
        stuck_timeout: Duration::from_secs(15),                // Stuck after 15s
        prompt_patterns: vec![
            r".*@.*:\S*\$ $".to_string(), // bash: user@host:/path$
            r".*# $".to_string(),         // root: #
            r"\$ $".to_string(),          // simple: $
        ],
        snapshot_history_size: 5,
    };

    info!("Creating terminal output monitor...");
    let mut monitor = TerminalOutputMonitor::with_config(ht_client.clone(), monitor_config).await?;

    // Start monitoring and get state transition receiver
    let mut state_receiver = monitor.start_monitoring();

    // Spawn monitoring task
    let ht_client_clone = ht_client.clone();
    let monitor_task = tokio::spawn(async move {
        if let Err(e) = monitor.run_monitoring_loop().await {
            warn!("Monitor loop error: {}", e);
        }
    });

    // Task to demonstrate various terminal commands
    let demo_task = tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;

        info!("Sending command: echo 'Hello, World!'");
        let _ = ht_client_clone.send_keys("echo 'Hello, World!'\r").await;

        sleep(Duration::from_secs(3)).await;

        info!("Sending long-running command: sleep 5");
        let _ = ht_client_clone.send_keys("sleep 5\r").await;

        sleep(Duration::from_secs(7)).await;

        info!("Sending command that generates continuous output: for i in {{1..10}}; do echo \"Line $i\"; sleep 0.5; done");
        let _ = ht_client_clone
            .send_keys("for i in {1..10}; do echo \"Line $i\"; sleep 0.5; done\r")
            .await;

        sleep(Duration::from_secs(8)).await;

        info!("Demo completed");
    });

    // Task to handle state transitions
    let state_handler_task = tokio::spawn(async move {
        let mut transition_count = 0;

        while let Some(transition) = state_receiver.recv().await {
            transition_count += 1;

            info!(
                "State Transition #{}: {} -> {} (at {:?})",
                transition_count,
                format_state(&transition.from),
                format_state(&transition.to),
                transition.timestamp
            );

            // Show some context from the terminal snapshot
            let lines: Vec<&str> = transition.snapshot.content.lines().collect();
            if let Some(last_line) = lines.last() {
                info!("Terminal context: '{}'", last_line);
            }

            // Example: React to specific state transitions
            match (&transition.from, &transition.to) {
                (AgentState::Active, AgentState::Wait) => {
                    info!("Command execution appears to be waiting for input or stuck");
                }
                (AgentState::Wait, AgentState::Active) => {
                    info!("Command execution resumed");
                }
                (AgentState::Active, AgentState::Idle) => {
                    info!("Command completed, back to shell prompt");
                }
                (AgentState::Idle, AgentState::Active) => {
                    info!("New command started");
                }
                _ => {}
            }

            // Stop after reasonable number of transitions for demo
            if transition_count >= 20 {
                info!("Received {} transitions, stopping demo", transition_count);
                break;
            }
        }
    });

    // Wait for demo to complete or timeout
    let result = timeout(Duration::from_secs(30), demo_task).await;

    match result {
        Ok(Ok(())) => info!("Demo completed successfully"),
        Ok(Err(e)) => warn!("Demo task error: {}", e),
        Err(_) => warn!("Demo timed out"),
    }

    // Give some time for final state transitions
    sleep(Duration::from_secs(2)).await;

    // Stop monitoring
    monitor_task.abort();
    state_handler_task.abort();

    // Shutdown HT client
    info!("Shutting down HT client...");
    ht_client.stop().await?;

    info!("Terminal output monitoring example completed");
    Ok(())
}

fn format_state(state: &AgentState) -> &'static str {
    match state {
        AgentState::Idle => "Idle",
        AgentState::Wait => "Wait",
        AgentState::Active => "Active",
    }
}
