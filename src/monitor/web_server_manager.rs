use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::agent;
use crate::ruler::config::MonitorConfig;
use crate::web_server::WebServer;

/// Web server manager responsible for starting and managing web servers for each agent
/// LEGACY: This functionality is now integrated directly into Agent struct
#[allow(dead_code)]
pub struct WebServerManager {
    pub agent_pool: Arc<agent::AgentPool>,
    pub monitor_config: MonitorConfig,
}

#[allow(dead_code)]
impl WebServerManager {
    pub fn new(agent_pool: Arc<agent::AgentPool>, monitor_config: MonitorConfig) -> Self {
        Self {
            agent_pool,
            monitor_config,
        }
    }

    /// Start web servers for all agents if enabled, and display URLs
    pub async fn start_web_servers(&self) -> Vec<JoinHandle<()>> {
        let mut web_server_handles = Vec::new();

        if self.monitor_config.web_ui.enabled {
            // Start web servers
            for i in 0..self.monitor_config.get_agent_pool_size() {
                let port = self.monitor_config.get_web_ui_port() + i as u16;
                let agent = self.agent_pool.get_agent_by_index(i);
                let web_server =
                    WebServer::new(port, self.monitor_config.web_ui.host.clone(), agent);

                let handle = tokio::spawn(async move {
                    if let Err(e) = web_server.start().await {
                        eprintln!("❌ Web server failed on port {}: {}", port, e);
                    }
                });
                web_server_handles.push(handle);
            }

            // Wait a moment for servers to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Display URLs
            self.display_web_ui_info();
        } else {
            println!("💡 Web UI disabled in configuration");
        }

        web_server_handles
    }

    fn display_web_ui_info(&self) {
        println!("🚀 Ready to monitor terminal commands...");
        println!("💡 Agent pool size: {}", self.agent_pool.size());

        for i in 0..self.monitor_config.get_agent_pool_size() {
            let port = self.monitor_config.get_web_ui_port() + i as u16;
            println!(
                "💡 Agent {} web UI: http://{}:{}",
                i + 1,
                self.monitor_config.web_ui.host,
                port
            );
        }
    }
}
