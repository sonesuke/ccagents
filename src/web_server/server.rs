use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, Response},
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

use super::websocket::handle_websocket;
use crate::agent::Agent;
use crate::web_ui::assets::AssetCache;

#[derive(Clone)]
pub struct WebServer {
    pub port: u16,
    pub host: String,
    pub agent: Arc<Agent>,
    pub asset_cache: AssetCache,
}

impl WebServer {
    pub fn new(port: u16, host: String, agent: Arc<Agent>) -> Self {
        Self {
            port,
            host,
            agent,
            asset_cache: AssetCache::new(),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let app = self.create_app();
        // Convert localhost to 127.0.0.1 for proper parsing
        let host = if self.host == "localhost" {
            "127.0.0.1"
        } else {
            &self.host
        };
        let addr: SocketAddr = format!("{}:{}", host, self.port).parse()?;

        info!(
            "🌐 Starting web server on http://{}:{}",
            self.host, self.port
        );
        println!("🌐 Web server binding to address: {}", addr);

        let listener = TcpListener::bind(addr).await?;
        println!("✅ Web server successfully bound to {}", addr);

        info!(
            "🚀 Web server ready and listening on http://{}:{}",
            self.host, self.port
        );

        axum::serve(listener, app).await?;

        Ok(())
    }

    fn create_app(&self) -> Router {
        Router::new()
            .route("/", get(serve_index))
            .route("/ws", get(websocket_handler))
            .with_state((self.agent.clone(), self.asset_cache.clone()))
            .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
    }
}

async fn serve_index(
    State((_, asset_cache)): State<(Arc<Agent>, AssetCache)>,
) -> Result<Html<String>, (StatusCode, String)> {
    info!("📄 Serving index.html to client");
    println!("📄 HTTP request for index page received");

    match asset_cache.get_index_html().await {
        Ok(content) => Ok(Html(content)),
        Err(e) => {
            tracing::error!("Failed to serve index.html: {}", e);
            Err((StatusCode::NOT_FOUND, "index.html not found".to_string()))
        }
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State((agent, _)): State<(Arc<Agent>, AssetCache)>,
) -> Response {
    info!("🔌 WebSocket upgrade request received");
    crate::debug_print!("🔌 WebSocket connection attempt");
    ws.on_upgrade(move |socket| handle_websocket(socket, agent))
}
