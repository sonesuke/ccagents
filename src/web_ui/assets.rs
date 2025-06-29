// Build-time embedded assets for the web terminal interface

use anyhow::Result;

// Embed index.html at build time
const INDEX_HTML: &str = include_str!("index.html");

#[derive(Clone, Default)]
pub struct AssetCache;

impl AssetCache {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_index_html(&self) -> Result<String> {
        Ok(INDEX_HTML.to_string())
    }
}
