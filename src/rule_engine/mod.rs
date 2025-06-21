pub mod compiled_rule;
pub mod decision;
pub mod rule_file;

pub use compiled_rule::CompiledRule;
pub use decision::decide_cmd;
pub use rule_file::{load_rules, CmdKind, RuleFile};

use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

pub struct RuleEngine {
    rules: Arc<RwLock<Vec<CompiledRule>>>,
    rules_path: std::path::PathBuf,
    _watcher: Option<RecommendedWatcher>,
}

impl std::fmt::Debug for RuleEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleEngine")
            .field("rules", &self.rules)
            .field("rules_path", &self.rules_path)
            .field("_watcher", &"<watcher>")
            .finish()
    }
}

impl RuleEngine {
    pub async fn new(rules_path: &str) -> Result<Self> {
        let path = std::path::PathBuf::from(rules_path);
        let rules = load_rules(&path)?;
        let rules = Arc::new(RwLock::new(rules));

        Ok(Self {
            rules,
            rules_path: path,
            _watcher: None,
        })
    }

    pub async fn get_rules(&self) -> Vec<CompiledRule> {
        self.rules.read().unwrap().clone()
    }

    pub async fn start_hot_reload(&mut self) -> Result<()> {
        let rules = self.rules.clone();
        let rules_path = self.rules_path.clone();

        let (tx, mut rx) = mpsc::channel(1);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        let _ = tx.blocking_send(());
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(&self.rules_path, RecursiveMode::NonRecursive)?;

        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                match load_rules(&rules_path) {
                    Ok(new_rules) => {
                        *rules.write().unwrap() = new_rules;
                        tracing::info!("Rules reloaded from {:?}", rules_path);
                    }
                    Err(e) => {
                        tracing::error!("Failed to reload rules: {}", e);
                    }
                }
            }
        });

        self._watcher = Some(watcher);
        tracing::info!("Hot reload started for {:?}", self.rules_path);
        Ok(())
    }

    pub fn register_rule(&mut self, rule: CompiledRule) -> Result<()> {
        self.rules.write().unwrap().push(rule);
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!(
            "Starting rule engine with {} rules",
            self.rules.read().unwrap().len()
        );
        self.start_hot_reload().await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping rule engine");
        Ok(())
    }
}

impl Clone for RuleEngine {
    fn clone(&self) -> Self {
        Self {
            rules: self.rules.clone(),
            rules_path: self.rules_path.clone(),
            _watcher: None,
        }
    }
}
