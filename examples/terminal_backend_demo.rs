use rule_agents::Ruler;
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Agent Demo Starting");

    // Demo 1: Create a Ruler with agents
    info!("=== Demo 1: Ruler with Agents ===");
    let mut ruler = match Ruler::new("examples/basic-rules.yaml").await {
        Ok(ruler) => {
            info!("Created Ruler successfully");
            ruler
        }
        Err(e) => {
            error!("Failed to create Ruler: {}", e);
            return Err(e.into());
        }
    };

    // Create an agent
    if let Err(e) = ruler.create_agent("demo-agent").await {
        error!("Failed to create agent: {}", e);
        return Err(e.into());
    }

    info!("Created demo-agent successfully");

    // Test agent waiting scenarios
    let scenarios = vec![
        ("demo-agent", "issue 123 detected in process"),
        ("demo-agent", "resume operation"),
        ("demo-agent", "unknown scenario"),
    ];

    for (agent_id, capture) in scenarios {
        tokio::time::sleep(Duration::from_millis(500)).await;

        match ruler.handle_waiting_state(agent_id, capture).await {
            Ok(()) => {
                info!("Successfully handled scenario: '{}'", capture);
            }
            Err(e) => {
                error!("Failed to handle scenario '{}': {}", capture, e);
            }
        }
    }

    // Demo 2: Session management
    info!("=== Demo 2: Session Management ===");
    let sessions = ruler.list_sessions().await;
    info!("Found {} sessions", sessions.len());

    // Demo 3: Health check
    info!("=== Demo 3: Health Check ===");
    match ruler.health_check().await {
        Ok(healthy) => {
            info!(
                "Health check result: {}",
                if healthy { "Healthy" } else { "Unhealthy" }
            );
        }
        Err(e) => {
            error!("Health check failed: {}", e);
        }
    }

    // Demo 4: Cleanup
    info!("=== Demo 4: Cleanup ===");
    if let Err(e) = ruler.force_cleanup_agent("demo-agent").await {
        error!("Cleanup failed: {}", e);
    } else {
        info!("Agent cleanup completed");
    }

    info!("Agent Demo Complete");
    Ok(())
}
