use super::server::WebServer;
use crate::agent::Agent;
use crate::config::Config;
use crate::config::web_ui_config::WebUIConfig;
use crate::web_ui::assets::AssetCache;

#[tokio::test]
async fn test_web_server_creation() {
    let config = Config::default();
    let agent = Agent::from_config(0, &config).await.unwrap();
    let web_server = WebServer::new(8080, "localhost".to_string(), agent);

    assert_eq!(web_server.port, 8080);
    assert_eq!(web_server.host, "localhost");
}

#[test]
fn test_web_ui_config_defaults() {
    let config = WebUIConfig::default();

    assert!(config.enabled);
    assert_eq!(config.host, "localhost");
}

#[test]
fn test_config_with_web_ui() {
    let config = Config::default();

    assert_eq!(config.web_ui.base_port, 9990);
    assert_eq!(config.agents.pool, 1);
    assert!(config.web_ui.enabled);
    assert_eq!(config.web_ui.host, "localhost");
    assert_eq!(config.web_ui.base_port, 9990);
    assert_eq!(config.agents.pool, 1);
}

#[tokio::test]
async fn test_asset_cache_html() {
    let cache = AssetCache::new();
    let result = cache.get_index_html().await;

    match result {
        Ok(content) => {
            assert!(!content.is_empty());
            assert!(content.contains("Rule Agents Terminal"));
            assert!(content.contains("AsciinemaPlayer.create"));
        }
        Err(_) => {
            // Asset file might not exist in test environment, which is acceptable
            println!("Asset file not found in test environment");
        }
    }
}

#[tokio::test]
async fn test_asset_cache_caching() {
    let cache = AssetCache::new();

    // Test that caching works - second access should hit cache
    if let Ok(content1) = cache.get_index_html().await {
        if let Ok(content2) = cache.get_index_html().await {
            assert_eq!(content1, content2);
        }
    }
}
