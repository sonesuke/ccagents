use rule_agents::{
    BackendType, TerminalBackendConfig, TerminalBackendFactory, TerminalBackendManager,
};
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Terminal Backend Demo Starting");

    // Demo 1: Create backend manager with auto-detection
    info!("=== Demo 1: Auto Backend Selection ===");
    let manager = match TerminalBackendManager::new_auto().await {
        Ok(manager) => {
            info!("Created backend manager with auto-detection");
            info!("Backend type: {}", manager.backend().backend_type());
            manager
        }
        Err(e) => {
            error!("Failed to create auto backend manager: {}", e);
            return Err(e.into());
        }
    };

    // Test backend availability
    let is_available = manager.is_backend_available().await;
    info!("Backend available: {}", is_available);

    if is_available {
        // Execute a simple command
        let result = manager
            .backend()
            .execute_command("echo 'Hello from backend!'")
            .await;
        match result {
            Ok(cmd_result) => {
                info!("Command executed successfully");
                info!("Output: {}", cmd_result.output);
                if let Some(exit_code) = cmd_result.exit_code {
                    info!("Exit code: {}", exit_code);
                }
            }
            Err(e) => {
                error!("Command execution failed: {}", e);
            }
        }

        // Take a snapshot if supported
        match manager.backend().take_snapshot().await {
            Ok(snapshot) => {
                info!("Snapshot taken successfully");
                info!("Terminal size: {}x{}", snapshot.width, snapshot.height);
                info!(
                    "Content preview: {}",
                    snapshot
                        .content
                        .lines()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            }
            Err(e) => {
                info!("Snapshot not supported by this backend: {}", e);
            }
        }
    }

    // Demo 2: Create direct backend explicitly
    info!("=== Demo 2: Direct Backend ===");
    let direct_config = TerminalBackendConfig::new()
        .with_backend_type(BackendType::Direct)
        .with_direct_timeout(Duration::from_secs(10));

    match TerminalBackendManager::new(direct_config).await {
        Ok(direct_manager) => {
            info!("Created direct backend successfully");

            // Test working directory operations
            if let Err(e) = direct_manager.backend().set_working_directory("/tmp").await {
                error!("Failed to set working directory: {}", e);
            } else {
                info!("Working directory set to /tmp");
            }

            // Execute directory listing
            match direct_manager
                .backend()
                .execute_command("pwd && ls -la")
                .await
            {
                Ok(result) => {
                    info!("Directory listing executed");
                    info!("Output:\n{}", result.output);
                }
                Err(e) => {
                    error!("Directory listing failed: {}", e);
                }
            }

            // Get environment variables
            match direct_manager.backend().get_environment().await {
                Ok(env_vars) => {
                    info!("Retrieved {} environment variables", env_vars.len());
                    // Show a few interesting ones
                    for key in ["HOME", "PATH", "USER", "SHELL"].iter() {
                        if let Some(value) = env_vars.get(*key) {
                            info!("{}={}", key, value);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get environment: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to create direct backend: {}", e);
        }
    }

    // Demo 3: Try HT backend
    info!("=== Demo 3: HT Backend ===");
    let ht_config = TerminalBackendConfig::new().with_backend_type(BackendType::Ht);

    match TerminalBackendManager::new(ht_config).await {
        Ok(ht_manager) => {
            info!("Created HT backend successfully");
            info!("Backend type: {}", ht_manager.backend().backend_type());

            // Test basic command execution
            match ht_manager.backend().execute_command("date").await {
                Ok(result) => {
                    info!("Date command executed");
                    info!("Output: {}", result.output.trim());
                }
                Err(e) => {
                    error!("Date command failed: {}", e);
                }
            }
        }
        Err(e) => {
            info!("HT backend not available: {}", e);
        }
    }

    // Demo 4: Factory usage
    info!("=== Demo 4: Factory Pattern ===");
    match TerminalBackendFactory::create_direct_only().await {
        Ok(direct_backend) => {
            info!(
                "Factory created direct backend: {}",
                direct_backend.backend_type()
            );

            // Quick test
            match direct_backend.execute_command("whoami").await {
                Ok(result) => {
                    info!("Current user: {}", result.output.trim());
                }
                Err(e) => {
                    error!("whoami command failed: {}", e);
                }
            }

            // Cleanup
            if let Err(e) = direct_backend.cleanup().await {
                error!("Cleanup failed: {}", e);
            }
        }
        Err(e) => {
            error!("Factory failed to create direct backend: {}", e);
        }
    }

    info!("Terminal Backend Demo Complete");
    Ok(())
}
