//! Mind: watch loop over patterns for WASM
//!
//! Intelligence = Pattern × Iteration × Memory
//! The Mind is the runtime; the PatternEngine is pure computation.

use super::log;
use super::store::WasmStore;
use crate::core::pattern::{Pattern, PatternDef};
use futures::StreamExt;
use nine_s_core::prelude::Scroll;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

/// The Mind: watches for scroll changes and applies patterns
pub struct Mind {
    store: Rc<WasmStore>,
    patterns: RefCell<Vec<Pattern>>,
    patterns_path: String,
}

impl Mind {
    pub fn new(store: Rc<WasmStore>) -> Self {
        Self {
            store,
            patterns: RefCell::new(Vec::new()),
            patterns_path: "/sys/patterns".to_string(),
        }
    }

    pub fn with_patterns_path(mut self, path: &str) -> Self {
        self.patterns_path = path.to_string();
        self
    }

    /// Load patterns from the patterns namespace
    pub async fn load_patterns(&self) -> Result<usize, String> {
        let paths = self.store.list(&self.patterns_path).await
            .map_err(|e| format!("Failed to list patterns: {:?}", e))?;

        let mut patterns = self.patterns.borrow_mut();
        patterns.clear();

        for path in paths {
            if let Ok(Some(scroll)) = self.store.read(&path).await {
                match serde_json::from_value::<PatternDef>(scroll.data.clone()) {
                    Ok(def) => {
                        match Pattern::compile(def) {
                            Ok(pattern) => {
                                log!("[Mind] Loaded pattern: {}", pattern.name);
                                patterns.push(pattern);
                            }
                            Err(e) => {
                                log!("[Mind] Failed to compile pattern at {}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        log!("[Mind] Invalid pattern at {}: {}", path, e);
                    }
                }
            }
        }

        Ok(patterns.len())
    }

    /// Add a pattern directly
    pub fn add_pattern(&self, def: PatternDef) -> Result<(), String> {
        let pattern = Pattern::compile(def)
            .map_err(|e| e.to_string())?;
        self.patterns.borrow_mut().push(pattern);
        Ok(())
    }

    /// Get pattern count
    pub fn pattern_count(&self) -> usize {
        self.patterns.borrow().len()
    }

    /// Apply all patterns to a scroll
    pub async fn apply(&self, scroll: &Scroll) -> Result<Vec<Scroll>, String> {
        let mut reactions = Vec::new();
        let patterns = self.patterns.borrow();

        for pattern in patterns.iter() {
            match pattern.apply(scroll, Some("wasm-mind")) {
                Ok(Some(reaction)) => {
                    log!("[Mind] Pattern '{}' matched {} → {}",
                        pattern.name, scroll.key, reaction.key);

                    // Write reaction to store
                    self.store.write(&reaction.key, reaction.data.clone()).await
                        .map_err(|e| format!("Failed to write reaction: {:?}", e))?;

                    reactions.push(reaction);
                }
                Ok(None) => {}
                Err(e) => {
                    log!("[Mind] Pattern '{}' error: {}", pattern.name, e);
                }
            }
        }

        Ok(reactions)
    }

    /// Run the mind: watch for changes and apply patterns
    pub fn run(self: Rc<Self>) {
        let mind = self.clone();
        let patterns_path = self.patterns_path.clone();

        spawn_local(async move {
            log!("[Mind] Starting watch loop...");

            // Watch everything
            let rx = match mind.store.watch("/**") {
                Ok(rx) => rx,
                Err(e) => {
                    log!("[Mind] Failed to start watch: {:?}", e);
                    return;
                }
            };

            let mut rx = rx;
            while let Some(scroll) = rx.next().await {
                // Skip pattern scrolls to avoid loops
                if scroll.key.starts_with(&patterns_path) {
                    continue;
                }

                // Skip scrolls produced by mind itself
                if scroll.metadata.produced_by.as_deref() == Some("wasm-mind") {
                    continue;
                }

                log!("[Mind] Change detected: {}", scroll.key);

                if let Err(e) = mind.apply(&scroll).await {
                    log!("[Mind] Error applying patterns: {}", e);
                }
            }
        });
    }
}
