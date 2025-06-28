use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::header,
    response::{Html, Json, Response},
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

use super::assets;
use super::websocket::handle_websocket;
use crate::agent::Agent;
use serde_json::json;

#[derive(Clone)]
pub struct WebServer {
    pub port: u16,
    pub host: String,
    pub agent: Arc<Agent>,
}

impl WebServer {
    pub fn new(port: u16, host: String, agent: Arc<Agent>) -> Self {
        Self { port, host, agent }
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
            .route("/styles/main.css", get(serve_css))
            .route("/scripts/terminal-client.js", get(serve_js))
            .route("/ws", get(websocket_handler))
            .route("/config", get(serve_config))
            .with_state(self.agent.clone())
            .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
    }
}

async fn serve_index() -> Html<&'static str> {
    info!("📄 Serving index.html to client");
    println!("📄 HTTP request for index page received");
    Html(assets::INDEX_HTML)
}

async fn serve_css() -> Response {
    info!("🎨 Serving main.css to client");
    Response::builder()
        .header(header::CONTENT_TYPE, "text/css")
        .body(assets::MAIN_CSS.into())
        .unwrap()
}

async fn serve_js() -> Response {
    info!("📜 Serving terminal-client.js to client");
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(assets::TERMINAL_CLIENT_JS.into())
        .unwrap()
}

async fn websocket_handler(ws: WebSocketUpgrade, State(agent): State<Arc<Agent>>) -> Response {
    info!("🔌 WebSocket upgrade request received");
    crate::debug_print!("🔌 WebSocket connection attempt");
    ws.on_upgrade(move |socket| handle_websocket(socket, agent))
}

async fn serve_config(State(agent): State<Arc<Agent>>) -> Json<serde_json::Value> {
    let (cols, rows) = agent.get_terminal_size();
    let debug = crate::DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed);
    Json(json!({
        "cols": cols,
        "rows": rows,
        "debug": debug
    }))
}
