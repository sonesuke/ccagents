use rule_agents::{MonitorConfig, TerminalOutputMonitor, TerminalSnapshot};
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Terminal Monitor Example Starting");

    // Demo 1: Create a monitor with default configuration
    info!("=== Demo 1: Default Monitor ===");
    let monitor = TerminalOutputMonitor::new("demo-agent".to_string());
    info!("Created terminal monitor for agent: demo-agent");
    info!("Initial state: {:?}", monitor.current_state());

    // Demo 2: Create a monitor with custom configuration
    info!("=== Demo 2: Custom Monitor Configuration ===");
    let monitor_config = MonitorConfig {
        change_detection_interval: Duration::from_millis(100),
        output_stable_duration: Duration::from_secs(1),
        prompt_patterns: vec![
            r"\$\s*$".to_string(), // Bash prompt
            r">\s*$".to_string(),  // CMD prompt
            r"#\s*$".to_string(),  // Root prompt
        ],
        max_snapshot_history: 5,
    };

    let mut custom_monitor =
        TerminalOutputMonitor::with_config("custom-agent".to_string(), monitor_config);
    info!("Created custom monitor for agent: custom-agent");

    // Demo 3: Start monitoring and simulate terminal snapshots
    info!("=== Demo 3: Monitor State Transitions ===");
    let mut state_rx = custom_monitor.start_monitoring();

    // Simulate different terminal states
    let test_snapshots = vec![
        // Idle state - showing bash prompt
        TerminalSnapshot {
            content: "user@host:~/project$ ".to_string(),
            cursor_position: Some((19, 0)),
            width: 80,
            height: 24,
        },
        // Active state - command running with output
        TerminalSnapshot {
            content: "user@host:~/project$ ls -la\ndrwxr-xr-x  5 user user 4096 Jan 15 10:30 .\ndrwxr-xr-x 10 user user 4096 Jan 15 10:25 ..\n".to_string(),
            cursor_position: Some((0, 3)),
            width: 80,
            height: 24,
        },
        // Wait state - command running but no new output
        TerminalSnapshot {
            content: "user@host:~/project$ sleep 10\n".to_string(),
            cursor_position: Some((0, 1)),
            width: 80,
            height: 24,
        },
        // Back to idle - command completed
        TerminalSnapshot {
            content: "user@host:~/project$ sleep 10\nuser@host:~/project$ ".to_string(),
            cursor_position: Some((19, 1)),
            width: 80,
            height: 24,
        },
    ];

    // Process snapshots and monitor state transitions
    for (i, snapshot) in test_snapshots.iter().enumerate() {
        info!("Processing snapshot {}", i + 1);

        if let Err(e) = custom_monitor.process_snapshot(snapshot.clone()).await {
            error!("Failed to process snapshot: {}", e);
        }

        // Check for state transitions
        match state_rx.try_recv() {
            Ok(transition) => {
                info!(
                    "State transition detected: {:?} -> {:?} at {:?}",
                    transition.from, transition.to, transition.timestamp
                );
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                info!("No state change");
            }
            Err(e) => {
                error!("Error receiving state transition: {}", e);
            }
        }

        info!("Current state: {:?}", custom_monitor.current_state());

        // Small delay between snapshots
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Demo 4: Monitor statistics
    info!("=== Demo 4: Monitor Statistics ===");
    let stats = custom_monitor.get_statistics();
    info!("Monitor statistics: {:?}", stats);

    // Demo 5: Stop monitoring
    info!("=== Demo 5: Stop Monitoring ===");
    custom_monitor.stop_monitoring();
    info!("Monitoring stopped");

    info!("Terminal Monitor Example Complete");
    Ok(())
}
