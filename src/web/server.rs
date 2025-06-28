use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade},
    response::{Html, Response},
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
        let addr: SocketAddr = format!("{}:{}", self.host, self.port).parse()?;

        info!("Starting web server on http://{}:{}", self.host, self.port);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    fn create_app(&self) -> Router {
        Router::new()
            .route("/", get(serve_index))
            .route("/ws", get(websocket_handler))
            .with_state(self.agent.clone())
            .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
    }
}

async fn serve_index() -> Html<&'static str> {
    Html(assets::INDEX_HTML)
}

async fn websocket_handler(ws: WebSocketUpgrade, State(agent): State<Arc<Agent>>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, agent))
}
