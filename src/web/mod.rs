pub mod assets;
pub mod server;
pub mod websocket;

#[cfg(test)]
mod tests;

pub use server::WebServer;
