//! Effects: /external/** side effects

use anyhow::Result;
use async_trait::async_trait;
use nine_s_core::prelude::*;
use nine_s_store::Store;
use serde_json::Value;
use std::sync::Arc;
use crate::core::paths::{mind as paths, origin, EFFECT_RESULT_TYPE};

#[async_trait]
pub trait EffectHandler: Send + Sync {
    fn watches(&self) -> &str;
    async fn execute(&self, scroll: &Scroll) -> Result<Value>;
}

#[derive(Debug, Clone)]
pub struct EffectConfig { pub process_existing: bool, pub origin: String }
impl Default for EffectConfig { fn default() -> Self { Self { process_existing: false, origin: origin::EFFECTS.into() } } }

pub struct EffectWorker {
    store: Arc<Store>,
    handlers: Vec<Box<dyn EffectHandler>>,
    config: EffectConfig,
}

impl EffectWorker {
    pub fn new(store: Store) -> Self { Self { store: Arc::new(store), handlers: Vec::new(), config: EffectConfig::default() } }
    pub fn with_config(mut self, config: EffectConfig) -> Self { self.config = config; self }
    pub fn add_handler(mut self, handler: Box<dyn EffectHandler>) -> Self { self.handlers.push(handler); self }

    pub async fn run(&self) -> Result<()> {
        let rx = self.store.watch(&WatchPattern::parse(&format!("{}/**", paths::EXTERNAL_PREFIX))?)?;
        if self.config.process_existing {
            for path in self.store.list(paths::EXTERNAL_PREFIX)? {
                if !path.contains(paths::RESULT_SUFFIX) { if let Some(s) = self.store.read(&path)? { self.process(&s).await; } }
            }
        }
        while let Ok(s) = rx.recv() {
            if s.key.contains(paths::RESULT_SUFFIX) || s.metadata.produced_by.as_deref() == Some(&self.config.origin) { continue; }
            self.process(&s).await;
        }
        Ok(())
    }

    async fn process(&self, scroll: &Scroll) {
        for h in &self.handlers {
            if scroll.key.starts_with(h.watches()) {
                let data = match h.execute(scroll).await {
                    Ok(v) => serde_json::json!({"success": true, "result": v}),
                    Err(e) => serde_json::json!({"success": false, "error": e.to_string()}),
                };
                let _ = self.store.write_scroll(Scroll { key: format!("{}{}", scroll.key, paths::RESULT_SUFFIX), type_: EFFECT_RESULT_TYPE.into(), metadata: Metadata::default().with_produced_by(&self.config.origin), data });
                return;
            }
        }
    }
}
