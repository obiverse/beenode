//! Mind: watch loop over patterns. Intelligence = Pattern × Iteration × Memory

use anyhow::Result;
use nine_s_core::prelude::*;
use nine_s_store::Store;
use std::collections::HashMap;
use std::sync::Arc;
use crate::core::paths::{mind as paths, origin};
use crate::core::pattern::Pattern;

fn is_reserved(path: &str) -> bool { path.ends_with(paths::RESERVED_SUFFIX) }

#[derive(Debug, Clone)]
pub struct MindConfig { pub process_existing: bool, pub origin: String }
impl Default for MindConfig { fn default() -> Self { Self { process_existing: false, origin: origin::MIND.into() } } }

pub struct Mind {
    store: Arc<Store>,
    config: MindConfig,
    patterns: Vec<Pattern>,
    pattern_versions: HashMap<String, u64>,
}

impl Mind {
    pub fn new(store: Store) -> Self { Self { store: Arc::new(store), config: MindConfig::default(), patterns: Vec::new(), pattern_versions: HashMap::new() } }
    pub fn with_config(store: Store, config: MindConfig) -> Self { Self { store: Arc::new(store), config, patterns: Vec::new(), pattern_versions: HashMap::new() } }

    pub async fn run(&mut self) -> Result<()> {
        self.reload_patterns()?;
        tracing::info!("Mind: {} patterns loaded", self.patterns.len());
        let rx = self.store.watch(&WatchPattern::parse("/**")?)?;
        if self.config.process_existing {
            for path in self.store.list("/")? {
                if !self.should_skip(&path) { if let Some(s) = self.store.read(&path)? { self.apply_patterns(&s)?; } }
            }
        }
        while let Ok(scroll) = rx.recv() {
            if self.should_skip(&scroll.key) { continue; }
            if scroll.key.starts_with(paths::PATTERNS_PREFIX) { if self.check_pattern_changed(&scroll) { self.reload_patterns()?; } continue; }
            if scroll.metadata.produced_by.as_deref() == Some(&self.config.origin) { continue; }
            self.apply_patterns(&scroll)?;
        }
        Ok(())
    }

    fn should_skip(&self, path: &str) -> bool { is_reserved(path) || path.starts_with(paths::PATTERNS_PREFIX) }

    fn check_pattern_changed(&mut self, scroll: &Scroll) -> bool {
        let prev = self.pattern_versions.get(&scroll.key).copied().unwrap_or(0);
        if scroll.metadata.version > prev { self.pattern_versions.insert(scroll.key.clone(), scroll.metadata.version); true } else { false }
    }

    fn apply_patterns(&self, scroll: &Scroll) -> Result<()> {
        for pattern in &self.patterns {
            if let Some(reaction) = pattern.apply(scroll, Some(&self.config.origin))? {
                tracing::info!("'{}': {} -> {}", pattern.name, scroll.key, reaction.key);
                self.store.write_scroll(reaction.clone())?;
                if let Some(then) = &pattern.then { self.cascade(then, &reaction)?; }
            }
        }
        Ok(())
    }

    fn cascade(&self, pattern_path: &str, scroll: &Scroll) -> Result<()> {
        let path = if pattern_path.starts_with('/') { pattern_path.to_string() } else { format!("{}/{}", paths::PATTERNS_PREFIX, pattern_path) };
        if let Some(ps) = self.store.read(&path)? {
            let p = Pattern::from_value(ps.data)?;
            if let Some(r) = p.apply(scroll, Some(&self.config.origin))? {
                self.store.write_scroll(r.clone())?;
                if let Some(next) = &p.then { self.cascade(next, &r)?; }
            }
        }
        Ok(())
    }

    pub fn reload_patterns(&mut self) -> Result<()> {
        self.patterns.clear();
        for path in self.store.list(paths::PATTERNS_PREFIX)? {
            if is_reserved(&path) { continue; }
            if let Some(scroll) = self.store.read(&path)? {
                self.pattern_versions.insert(path.clone(), scroll.metadata.version);
                if let Ok(p) = Pattern::from_value(scroll.data) { self.patterns.push(p); }
            }
        }
        Ok(())
    }

    pub fn load_patterns(&self) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();
        for path in self.store.list(paths::PATTERNS_PREFIX)? { if is_reserved(&path) { continue; } if let Some(s) = self.store.read(&path)? { if let Ok(p) = Pattern::from_value(s.data) { patterns.push(p); } } }
        Ok(patterns)
    }

    pub fn store(&self) -> &Store { &self.store }
}
