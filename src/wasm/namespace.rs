//! WASM Namespace implementations
//!
//! Browser namespaces:
//! - IndexedDB: Persistent local storage
//! - Memory: Fast ephemeral cache

use nine_s_core::prelude::{Metadata, Scroll};
use futures::channel::mpsc;
use indexed_db_futures::prelude::*;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use super::account::AccountNamespace;
use super::auth::AuthNamespace;
#[cfg(feature = "bitcoin")]
use super::identity::IdentityNamespace;

/// Result type for namespace operations
pub type NamespaceResult<T> = Result<T, NamespaceError>;

/// Namespace errors
#[derive(Debug)]
pub enum NamespaceError {
    NotFound(String),
    IndexedDb(String),
    Serialization(String),
    WatchUnavailable,
    Other(String),
}

impl std::fmt::Display for NamespaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NamespaceError::NotFound(s) => write!(f, "Not found: {}", s),
            NamespaceError::IndexedDb(s) => write!(f, "IndexedDB error: {}", s),
            NamespaceError::Serialization(s) => write!(f, "Serialization error: {}", s),
            NamespaceError::WatchUnavailable => write!(f, "Watch not available"),
            NamespaceError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<serde_json::Error> for NamespaceError {
    fn from(e: serde_json::Error) -> Self {
        NamespaceError::Serialization(e.to_string())
    }
}

// =============================================================================
// MEMORY NAMESPACE
// =============================================================================

/// In-memory namespace for fast ephemeral storage
#[derive(Clone)]
pub struct MemoryNamespace {
    scrolls: Rc<RefCell<HashMap<String, Scroll>>>,
    watchers: Rc<RefCell<Vec<mpsc::UnboundedSender<Scroll>>>>,
}

impl MemoryNamespace {
    pub fn new() -> Self {
        Self {
            scrolls: Rc::new(RefCell::new(HashMap::new())),
            watchers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub async fn read(&self, path: &str) -> NamespaceResult<Option<Scroll>> {
        let scrolls = self.scrolls.borrow();
        Ok(scrolls.get(path).cloned())
    }

    pub async fn write(&self, path: &str, data: Value) -> NamespaceResult<Scroll> {
        let mut scrolls = self.scrolls.borrow_mut();

        let version = scrolls
            .get(path)
            .map(|s| s.metadata.version + 1)
            .unwrap_or(1);

        // Extract _type from data if present, otherwise use generic
        let type_ = data
            .get("_type")
            .and_then(|v| v.as_str())
            .unwrap_or("generic@v1")
            .to_string();

        let scroll = Scroll {
            key: path.to_string(),
            type_,
            metadata: Metadata::default().with_version(version),
            data,
        };

        scrolls.insert(path.to_string(), scroll.clone());
        drop(scrolls); // Release borrow before notifying

        // Notify watchers
        let watchers = self.watchers.borrow();
        for tx in watchers.iter() {
            let _ = tx.unbounded_send(scroll.clone());
        }

        Ok(scroll)
    }

    pub async fn list(&self, prefix: &str) -> NamespaceResult<Vec<String>> {
        let scrolls = self.scrolls.borrow();
        let paths: Vec<String> = scrolls
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(paths)
    }

    pub fn watch(&self, _pattern: &str) -> NamespaceResult<mpsc::UnboundedReceiver<Scroll>> {
        let (tx, rx) = mpsc::unbounded();
        let mut watchers = self.watchers.borrow_mut();
        watchers.push(tx);
        Ok(rx)
    }

    pub async fn close(&self) -> NamespaceResult<()> {
        Ok(())
    }
}

impl Default for MemoryNamespace {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// INDEXEDDB NAMESPACE
// =============================================================================

const STORE_NAME: &str = "scrolls";

/// IndexedDB namespace for persistent browser storage
#[derive(Clone)]
pub struct IndexedDbNamespace {
    db_name: String,
    db: Rc<RefCell<Option<IdbDatabase>>>,
    watchers: Rc<RefCell<Vec<mpsc::UnboundedSender<Scroll>>>>,
}

impl IndexedDbNamespace {
    pub fn new(db_name: &str) -> Self {
        Self {
            db_name: db_name.to_string(),
            db: Rc::new(RefCell::new(None)),
            watchers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub async fn open(db_name: &str) -> NamespaceResult<Self> {
        let ns = Self::new(db_name);
        ns.ensure_db().await?;
        Ok(ns)
    }

    async fn ensure_db(&self) -> NamespaceResult<()> {
        if self.db.borrow().is_some() {
            return Ok(());
        }

        let mut db_req = IdbDatabase::open_u32(&self.db_name, 1)
            .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

        db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
            if !evt.db().object_store_names().any(|n| n == STORE_NAME) {
                evt.db().create_object_store(STORE_NAME)?;
            }
            Ok(())
        }));

        let db = db_req.await
            .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

        *self.db.borrow_mut() = Some(db);
        Ok(())
    }

    pub async fn read(&self, path: &str) -> NamespaceResult<Option<Scroll>> {
        self.ensure_db().await?;

        let value = {
            let db_ref = self.db.borrow();
            let db = db_ref.as_ref()
                .ok_or_else(|| NamespaceError::IndexedDb("Database not open".to_string()))?;

            let tx = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readonly)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

            let store = tx.object_store(STORE_NAME)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

            store.get_owned(path)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?
        }.await
            .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

        match value {
            Some(js_val) => {
                let scroll: Scroll = serde_wasm_bindgen::from_value(js_val)
                    .map_err(|e| NamespaceError::Serialization(e.to_string()))?;
                Ok(Some(scroll))
            }
            None => Ok(None),
        }
    }

    pub async fn write(&self, path: &str, data: Value) -> NamespaceResult<Scroll> {
        self.ensure_db().await?;

        // Read existing to get version
        let existing = self.read(path).await?;
        let version = existing.map(|s| s.metadata.version + 1).unwrap_or(1);

        // Extract _type from data if present, otherwise use generic
        let type_ = data
            .get("_type")
            .and_then(|v| v.as_str())
            .unwrap_or("generic@v1")
            .to_string();

        let scroll = Scroll {
            key: path.to_string(),
            type_,
            metadata: Metadata::default().with_version(version),
            data,
        };

        // Serialize scroll before borrowing db
        let js_val = serde_wasm_bindgen::to_value(&scroll)
            .map_err(|e| NamespaceError::Serialization(e.to_string()))?;

        {
            let db_ref = self.db.borrow();
            let db = db_ref.as_ref()
                .ok_or_else(|| NamespaceError::IndexedDb("Database not open".to_string()))?;

            let tx = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readwrite)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

            let store = tx.object_store(STORE_NAME)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

            store.put_key_val_owned(path, &js_val)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?
        }.await
            .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

        // Notify watchers
        let watchers = self.watchers.borrow();
        for tx in watchers.iter() {
            let _ = tx.unbounded_send(scroll.clone());
        }

        Ok(scroll)
    }

    pub async fn list(&self, prefix: &str) -> NamespaceResult<Vec<String>> {
        self.ensure_db().await?;

        let keys = {
            let db_ref = self.db.borrow();
            let db = db_ref.as_ref()
                .ok_or_else(|| NamespaceError::IndexedDb("Database not open".to_string()))?;

            let tx = db.transaction_on_one_with_mode(STORE_NAME, IdbTransactionMode::Readonly)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

            let store = tx.object_store(STORE_NAME)
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

            store.get_all_keys()
                .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?
        }.await
            .map_err(|e| NamespaceError::IndexedDb(format!("{:?}", e)))?;

        let mut paths = Vec::new();
        for key in keys.iter() {
            if let Some(path) = key.as_string() {
                if path.starts_with(prefix) {
                    paths.push(path);
                }
            }
        }

        Ok(paths)
    }

    pub fn watch(&self, _pattern: &str) -> NamespaceResult<mpsc::UnboundedReceiver<Scroll>> {
        let (tx, rx) = mpsc::unbounded();
        let mut watchers = self.watchers.borrow_mut();
        watchers.push(tx);
        Ok(rx)
    }

    pub async fn close(&self) -> NamespaceResult<()> {
        if let Some(db) = self.db.borrow().as_ref() {
            db.close();
        }
        Ok(())
    }
}

// =============================================================================
// NAMESPACE ENUM (for Store routing without dyn traits)
// =============================================================================

/// Namespace enum - allows heterogeneous namespace storage without dyn traits
#[derive(Clone)]
pub enum Namespace {
    Memory(MemoryNamespace),
    IndexedDb(IndexedDbNamespace),
    Auth(AuthNamespace),
    Account(AccountNamespace),
    #[cfg(feature = "bitcoin")]
    Identity(IdentityNamespace),
}

impl Namespace {
    pub async fn read(&self, path: &str) -> NamespaceResult<Option<Scroll>> {
        match self {
            Namespace::Memory(ns) => ns.read(path).await,
            Namespace::IndexedDb(ns) => ns.read(path).await,
            Namespace::Auth(ns) => ns.read(path).await,
            Namespace::Account(ns) => ns.read(path).await,
            #[cfg(feature = "bitcoin")]
            Namespace::Identity(ns) => ns.read(path).await,
        }
    }

    pub async fn write(&self, path: &str, data: Value) -> NamespaceResult<Scroll> {
        match self {
            Namespace::Memory(ns) => ns.write(path, data).await,
            Namespace::IndexedDb(ns) => ns.write(path, data).await,
            Namespace::Auth(ns) => ns.write(path, data).await,
            Namespace::Account(ns) => ns.write(path, data).await,
            #[cfg(feature = "bitcoin")]
            Namespace::Identity(ns) => ns.write(path, data).await,
        }
    }

    pub async fn list(&self, prefix: &str) -> NamespaceResult<Vec<String>> {
        match self {
            Namespace::Memory(ns) => ns.list(prefix).await,
            Namespace::IndexedDb(ns) => ns.list(prefix).await,
            Namespace::Auth(ns) => ns.list(prefix).await,
            Namespace::Account(ns) => ns.list(prefix).await,
            #[cfg(feature = "bitcoin")]
            Namespace::Identity(ns) => ns.list(prefix).await,
        }
    }

    pub fn watch(&self, pattern: &str) -> NamespaceResult<mpsc::UnboundedReceiver<Scroll>> {
        match self {
            Namespace::Memory(ns) => ns.watch(pattern),
            Namespace::IndexedDb(ns) => ns.watch(pattern),
            Namespace::Auth(ns) => ns.watch(pattern),
            Namespace::Account(ns) => ns.watch(pattern),
            #[cfg(feature = "bitcoin")]
            Namespace::Identity(ns) => ns.watch(pattern),
        }
    }

    pub async fn close(&self) -> NamespaceResult<()> {
        match self {
            Namespace::Memory(ns) => ns.close().await,
            Namespace::IndexedDb(ns) => ns.close().await,
            Namespace::Auth(ns) => ns.close().await,
            Namespace::Account(ns) => ns.close().await,
            #[cfg(feature = "bitcoin")]
            Namespace::Identity(ns) => ns.close().await,
        }
    }
}
