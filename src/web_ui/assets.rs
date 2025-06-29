// Runtime asset serving with caching for the web terminal interface

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::fs;

#[derive(Clone)]
pub struct AssetCache {
    cache: Arc<RwLock<HashMap<String, String>>>,
    assets_path: String,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            assets_path: "assets/web".to_string(),
        }
    }

    pub async fn get_asset(&self, path: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(content) = cache.get(path) {
                return Ok(content.clone());
            }
        }

        // Load from filesystem
        let full_path = Path::new(&self.assets_path).join(path);
        let content = fs::read_to_string(&full_path).await?;

        // Store in cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(path.to_string(), content.clone());
        }

        Ok(content)
    }

    pub async fn get_index_html(&self) -> Result<String> {
        self.get_asset("index.html").await
    }

    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}
