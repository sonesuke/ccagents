use crate::ruler::rule_loader::load_rules;
use crate::ruler::rule_types::CompiledRule;
use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HotReloader {
    rules: Arc<RwLock<Vec<CompiledRule>>>,
    _watcher: RecommendedWatcher,
}

impl std::fmt::Debug for HotReloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotReloader")
            .field("rules", &"<rules>")
            .field("_watcher", &"<watcher>")
            .finish()
    }
}

impl HotReloader {
    pub async fn new(rules_path: &str) -> Result<Self> {
        // Load initial rules using existing function from #3
        let initial_rules = load_rules(Path::new(rules_path))?;
        let rules = Arc::new(RwLock::new(initial_rules));

        // Set up file watcher
        let rules_clone = rules.clone();
        let path_clone = rules_path.to_string();

        let watcher = notify::recommended_watcher(move |res: NotifyResult<notify::Event>| {
            if res.is_ok() {
                let rules_ref = rules_clone.clone();
                let path_ref = path_clone.clone();
                // Handle the case where tokio runtime might not be available
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        reload_rules(rules_ref, path_ref).await;
                    });
                }
            }
        })?;

        let mut watcher = watcher;
        watcher.watch(Path::new(rules_path), RecursiveMode::NonRecursive)?;

        Ok(HotReloader {
            rules,
            _watcher: watcher,
        })
    }

    pub async fn get_rules(&self) -> Vec<CompiledRule> {
        self.rules.read().await.clone()
    }
}

async fn reload_rules(rules: Arc<RwLock<Vec<CompiledRule>>>, path: String) {
    // Debouncing - wait for file operations to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    match load_rules(Path::new(&path)) {
        Ok(new_rules) => {
            let mut rules_guard = rules.write().await;
            *rules_guard = new_rules;
            println!("✅ Rules reloaded successfully from {}", path);
        }
        Err(e) => {
            eprintln!("❌ Failed to reload rules: {}", e);
            eprintln!("   Keeping existing rules active");
        }
    }
}
