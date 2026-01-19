//! BeeNode: The browser as a full 9S node
//!
//! Exposes the node to JavaScript via wasm-bindgen
//!
//! Architecture:
//! - BeeNode: JS-facing API (5 frozen ops + Mind)
//! - WasmStore: Platform substrate (IndexedDB/Memory)
//! - Mind: Pattern engine runtime (watch loop)
//! - Pattern: Pure computation (no I/O)

use super::log;
use super::mind::Mind;
use super::store::WasmStore;
use crate::core::bse::{self, BSEEngine, BSENode, Pipeline};
use crate::core::pattern::{Pattern, PatternDef};
use nine_s_core::prelude::Scroll;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// JS-friendly wrapper for Scroll
#[wasm_bindgen]
pub struct JsScroll {
    inner: Scroll,
}

#[wasm_bindgen]
impl JsScroll {
    #[wasm_bindgen(getter)]
    pub fn key(&self) -> String {
        self.inner.key.clone()
    }

    #[wasm_bindgen(getter, js_name = "type")]
    pub fn type_(&self) -> String {
        self.inner.type_.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u64 {
        self.inner.metadata.version
    }

    #[wasm_bindgen(getter)]
    pub fn data(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        use serde::Serialize;
        self.inner.data.serialize(&serializer).unwrap_or(JsValue::NULL)
    }

    /// Get the full scroll as JSON
    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        use serde::Serialize;
        self.inner.serialize(&serializer).unwrap_or(JsValue::NULL)
    }
}

impl From<Scroll> for JsScroll {
    fn from(scroll: Scroll) -> Self {
        Self { inner: scroll }
    }
}

/// BeeNode: Browser-native 9S node with JS bindings
#[wasm_bindgen]
pub struct BeeNode {
    store: Rc<WasmStore>,
    patterns: RefCell<Vec<Pattern>>,
    mind: RefCell<Option<Rc<Mind>>>,
}

#[wasm_bindgen]
impl BeeNode {
    /// Create a new node with memory storage (for testing)
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        log!("[BeeNode] Creating with memory storage");
        Self {
            store: Rc::new(WasmStore::new()),
            patterns: RefCell::new(Vec::new()),
            mind: RefCell::new(None),
        }
    }

    /// Create a node with IndexedDB storage
    #[wasm_bindgen(js_name = "withIndexedDb")]
    pub async fn with_indexeddb(db_name: &str) -> Result<BeeNode, JsValue> {
        log!("[BeeNode] Creating with IndexedDB: {}", db_name);

        let store = WasmStore::with_indexeddb(db_name).await
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

        Ok(Self {
            store: Rc::new(store),
            patterns: RefCell::new(Vec::new()),
            mind: RefCell::new(None),
        })
    }

    // =========================================================================
    // THE 5 FROZEN OPERATIONS
    // =========================================================================

    /// Read a scroll by path
    #[wasm_bindgen]
    pub async fn read(&self, path: &str) -> Result<JsValue, JsValue> {
        match self.store.read(path).await {
            Ok(Some(scroll)) => {
                let js_scroll = JsScroll::from(scroll);
                Ok(js_scroll.to_json())
            }
            Ok(None) => Ok(JsValue::NULL),
            Err(e) => Err(JsValue::from_str(&format!("{}", e))),
        }
    }

    /// Write data to a path
    #[wasm_bindgen]
    pub async fn write(&self, path: &str, data: JsValue) -> Result<JsValue, JsValue> {
        let value: Value = serde_wasm_bindgen::from_value(data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        match self.store.write(path, value).await {
            Ok(scroll) => {
                let js_scroll = JsScroll::from(scroll);
                Ok(js_scroll.to_json())
            }
            Err(e) => Err(JsValue::from_str(&format!("{}", e))),
        }
    }

    /// List paths under a prefix
    #[wasm_bindgen]
    pub async fn list(&self, prefix: &str) -> Result<JsValue, JsValue> {
        match self.store.list(prefix).await {
            Ok(paths) => {
                serde_wasm_bindgen::to_value(&paths)
                    .map_err(|e| JsValue::from_str(&e.to_string()))
            }
            Err(e) => Err(JsValue::from_str(&format!("{}", e))),
        }
    }

    /// Watch for changes (returns subscription ID)
    #[wasm_bindgen]
    pub fn watch(&self, pattern: &str, callback: js_sys::Function) -> Result<u32, JsValue> {
        let rx = self.store.watch(pattern)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

        // Spawn task to forward changes to callback
        let this = JsValue::NULL;
        wasm_bindgen_futures::spawn_local(async move {
            use futures::StreamExt;
            let mut rx = rx;
            while let Some(scroll) = rx.next().await {
                let js_scroll = JsScroll::from(scroll);
                let _ = callback.call1(&this, &js_scroll.to_json());
            }
        });

        // Return dummy subscription ID
        Ok(1)
    }

    /// Close the node
    #[wasm_bindgen]
    pub async fn close(&self) -> Result<(), JsValue> {
        self.store.close().await
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    }

    // =========================================================================
    // PATTERNS (using shared core::pattern)
    // =========================================================================

    /// Add a pattern to the node
    #[wasm_bindgen(js_name = "addPattern")]
    pub fn add_pattern(&self, pattern_json: JsValue) -> Result<(), JsValue> {
        let def: PatternDef = serde_wasm_bindgen::from_value(pattern_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let pattern = Pattern::compile(def)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        self.patterns.borrow_mut().push(pattern);
        Ok(())
    }

    /// Apply patterns to a scroll manually
    #[wasm_bindgen(js_name = "applyPatterns")]
    pub fn apply_patterns(&self, scroll_json: JsValue) -> Result<JsValue, JsValue> {
        let scroll: Scroll = serde_wasm_bindgen::from_value(scroll_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let patterns = self.patterns.borrow();
        let mut reactions = Vec::new();

        for pattern in patterns.iter() {
            if let Ok(Some(reaction)) = pattern.apply(&scroll, Some("wasm")) {
                reactions.push(reaction);
            }
        }

        serde_wasm_bindgen::to_value(&reactions)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get pattern count
    #[wasm_bindgen(js_name = "patternCount")]
    pub fn pattern_count(&self) -> usize {
        self.patterns.borrow().len()
    }

    // =========================================================================
    // MIND (reactive pattern engine)
    // =========================================================================

    /// Initialize the Mind with patterns from a path
    /// Returns the number of patterns loaded
    #[wasm_bindgen(js_name = "initMind")]
    pub async fn init_mind(&self, patterns_path: &str) -> Result<u32, JsValue> {
        log!("[BeeNode] Initializing Mind with patterns from {}", patterns_path);

        let mind = Mind::new(self.store.clone())
            .with_patterns_path(patterns_path);

        let count = mind.load_patterns().await
            .map_err(|e| JsValue::from_str(&e))?;

        *self.mind.borrow_mut() = Some(Rc::new(mind));

        log!("[BeeNode] Mind loaded {} patterns", count);
        Ok(count as u32)
    }

    /// Start the Mind watch loop (patterns react to scroll changes)
    #[wasm_bindgen(js_name = "runMind")]
    pub fn run_mind(&self) -> Result<(), JsValue> {
        let mind_opt = self.mind.borrow();
        if let Some(mind) = mind_opt.as_ref() {
            log!("[BeeNode] Starting Mind watch loop...");
            mind.clone().run();
            Ok(())
        } else {
            Err(JsValue::from_str("Mind not initialized. Call initMind first."))
        }
    }

    /// Apply patterns via Mind (async, writes reactions to store)
    #[wasm_bindgen(js_name = "applyMindPatterns")]
    pub async fn apply_mind_patterns(&self, scroll_json: JsValue) -> Result<JsValue, JsValue> {
        let scroll: Scroll = serde_wasm_bindgen::from_value(scroll_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mind_opt = self.mind.borrow();
        if let Some(mind) = mind_opt.as_ref() {
            let mind = mind.clone();
            drop(mind_opt); // Release borrow before await

            let reactions = mind.apply(&scroll).await
                .map_err(|e| JsValue::from_str(&e))?;

            serde_wasm_bindgen::to_value(&reactions)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Ok(JsValue::from(js_sys::Array::new()))
        }
    }

    // =========================================================================
    // BSE (Block Structural Expressions)
    // Pike's SRE adapted for UI rendering
    // =========================================================================

    /// Parse a BSE DSL string to a pipeline
    /// Example: "x/type=hero/ c/HeroBlock/"
    #[wasm_bindgen(js_name = "parseBSE")]
    pub fn parse_bse(&self, dsl: &str) -> Result<JsValue, JsValue> {
        let pipeline = bse::parse_dsl(dsl)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&pipeline)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Evaluate a BSE pipeline against source data
    /// Returns BSENode[] for rendering
    #[wasm_bindgen(js_name = "evaluateBSE")]
    pub fn evaluate_bse(&self, pipeline_json: JsValue, source_json: JsValue) -> Result<JsValue, JsValue> {
        let pipeline: Pipeline = serde_wasm_bindgen::from_value(pipeline_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let source: Vec<Value> = serde_wasm_bindgen::from_value(source_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let nodes = BSEEngine::evaluate(&pipeline, &source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&nodes)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Evaluate a BSE DSL string against source data (convenience method)
    /// Combines parse + evaluate in one call
    #[wasm_bindgen(js_name = "queryBSE")]
    pub fn query_bse(&self, dsl: &str, source_json: JsValue) -> Result<JsValue, JsValue> {
        let pipeline = bse::parse_dsl(dsl)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let source: Vec<Value> = serde_wasm_bindgen::from_value(source_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let nodes = BSEEngine::evaluate(&pipeline, &source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&nodes)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Query scrolls from store and evaluate with BSE
    /// path_prefix: scroll path prefix (e.g., "/content/blog")
    /// dsl: BSE pipeline (e.g., "x/type=post/ o/date,desc/ n/5/ c/PostCard/")
    #[wasm_bindgen(js_name = "queryScrollsBSE")]
    pub async fn query_scrolls_bse(&self, path_prefix: &str, dsl: &str) -> Result<JsValue, JsValue> {
        // Get all scrolls under prefix
        let paths = self.store.list(path_prefix).await
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

        // Read all scrolls
        let mut source: Vec<Value> = Vec::new();
        for path in paths {
            if let Ok(Some(scroll)) = self.store.read(&path).await {
                // Include the scroll data with its path
                let mut data = scroll.data.clone();
                if let Value::Object(ref mut obj) = data {
                    obj.insert("_path".to_string(), Value::String(scroll.key.clone()));
                    obj.insert("_type".to_string(), Value::String(scroll.type_.clone()));
                }
                source.push(data);
            }
        }

        // Parse and evaluate BSE
        let pipeline = bse::parse_dsl(dsl)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let nodes = BSEEngine::evaluate(&pipeline, &source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&nodes)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for BeeNode {
    fn default() -> Self {
        Self::new()
    }
}
