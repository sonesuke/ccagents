use super::server::WebServer;
use crate::agent::Agent;
use crate::ruler::config::{MonitorConfig, WebUIConfig};
use std::sync::Arc;

#[tokio::test]
async fn test_web_server_creation() {
    let agent = Arc::new(Agent::new("test-agent".to_string(), true, 9999).await.unwrap());
    let web_server = WebServer::new(8080, "localhost".to_string(), agent);
    
    assert_eq!(web_server.port, 8080);
    assert_eq!(web_server.host, "localhost");
}

#[test]
fn test_web_ui_config_defaults() {
    let config = WebUIConfig::default();
    
    assert!(config.enabled);
    assert_eq!(config.host, "localhost");
    assert_eq!(config.theme, "default");
}

#[test]
fn test_monitor_config_with_web_ui() {
    let config = MonitorConfig::default();
    
    assert_eq!(config.base_port, 9990);
    assert_eq!(config.agent_pool_size, 1);
    assert!(config.web_ui.enabled);
    assert_eq!(config.web_ui.host, "localhost");
}

#[test]
fn test_assets_html_not_empty() {
    assert!(!crate::web::assets::INDEX_HTML.is_empty());
    assert!(crate::web::assets::INDEX_HTML.contains("Rule Agents Terminal"));
    assert!(crate::web::assets::INDEX_HTML.contains("WebSocket"));
}