//! WasmStore: Mounting namespaces at paths (browser edition)
//!
//! Like Plan 9's bind/mount, the Store routes paths to namespaces.

use super::namespace::{IndexedDbNamespace, MemoryNamespace, Namespace, NamespaceResult};
use super::account::AccountNamespace;
use super::auth::{AuthNamespace, AuthStorage, WasmAuth};
#[cfg(feature = "bitcoin")]
use super::identity::IdentityNamespace;
use nine_s_core::prelude::Scroll;
use futures::channel::mpsc;
use serde_json::Value;
use std::collections::BTreeMap;

/// WasmStore routes paths to mounted namespaces
#[derive(Clone)]
pub struct WasmStore {
    mounts: BTreeMap<String, Namespace>,
    default_ns: Namespace,
}

impl WasmStore {
    /// Create a new store with memory as default namespace
    pub fn new() -> Self {
        let auth = WasmAuth::new();
        let auth_ns = AuthNamespace::new(auth.clone());
        let account_ns = AccountNamespace::new(auth.clone(), None);
        let mut mounts = BTreeMap::from([
            ("/system/account".to_string(), Namespace::Account(account_ns)),
            ("/system/auth".to_string(), Namespace::Auth(auth_ns)),
        ]);

        #[cfg(feature = "bitcoin")]
        {
            let identity_ns = IdentityNamespace::new(auth);
            mounts.insert("/system/identity".to_string(), Namespace::Identity(identity_ns));
        }

        Self {
            mounts,
            default_ns: Namespace::Memory(MemoryNamespace::new()),
        }
    }

    /// Create a store with IndexedDB as default
    pub async fn with_indexeddb(db_name: &str) -> NamespaceResult<Self> {
        let idb = IndexedDbNamespace::open(db_name).await?;
        let auth = WasmAuth::new();
        let auth_db = format!("{}__auth", db_name);
        let storage = AuthStorage::open(&auth_db).await?;
        let auth_ns = AuthNamespace::with_storage(storage.clone(), auth.clone()).await?;
        let account_ns = AccountNamespace::new(auth.clone(), Some(storage));
        let mut mounts = BTreeMap::from([
            ("/system/account".to_string(), Namespace::Account(account_ns)),
            ("/system/auth".to_string(), Namespace::Auth(auth_ns)),
        ]);

        #[cfg(feature = "bitcoin")]
        {
            let identity_ns = IdentityNamespace::new(auth);
            mounts.insert("/system/identity".to_string(), Namespace::Identity(identity_ns));
        }

        Ok(Self {
            mounts,
            default_ns: Namespace::IndexedDb(idb),
        })
    }

    /// Mount a memory namespace at a path prefix
    pub fn mount_memory(&mut self, prefix: &str) {
        self.mounts.insert(prefix.to_string(), Namespace::Memory(MemoryNamespace::new()));
    }

    /// Mount an IndexedDB namespace at a path prefix
    pub async fn mount_indexeddb(&mut self, prefix: &str, db_name: &str) -> NamespaceResult<()> {
        let idb = IndexedDbNamespace::open(db_name).await?;
        self.mounts.insert(prefix.to_string(), Namespace::IndexedDb(idb));
        Ok(())
    }

    /// Mount a namespace at a path prefix
    pub fn mount(&mut self, prefix: &str, namespace: Namespace) {
        self.mounts.insert(prefix.to_string(), namespace);
    }

    /// Find the namespace for a path (longest prefix match)
    fn route(&self, path: &str) -> (&str, &Namespace) {
        // Find longest matching prefix
        for (prefix, ns) in self.mounts.iter().rev() {
            if path.starts_with(prefix) {
                return (prefix, ns);
            }
        }
        ("", &self.default_ns)
    }

    /// Strip the mount prefix from a path
    fn strip_prefix<'a>(&self, path: &'a str, prefix: &str) -> &'a str {
        if prefix.is_empty() {
            path
        } else {
            path.strip_prefix(prefix).unwrap_or(path)
        }
    }

    // =========================================================================
    // THE 5 FROZEN OPERATIONS
    // =========================================================================

    pub async fn read(&self, path: &str) -> NamespaceResult<Option<Scroll>> {
        let (prefix, ns) = self.route(path);
        let local_path = self.strip_prefix(path, prefix);
        ns.read(local_path).await
    }

    pub async fn write(&self, path: &str, data: Value) -> NamespaceResult<Scroll> {
        let (prefix, ns) = self.route(path);
        let local_path = self.strip_prefix(path, prefix);
        let mut scroll = ns.write(local_path, data).await?;
        // Restore full path in returned scroll
        scroll.key = path.to_string();
        Ok(scroll)
    }

    pub async fn list(&self, prefix: &str) -> NamespaceResult<Vec<String>> {
        let (mount_prefix, ns) = self.route(prefix);
        let local_prefix = self.strip_prefix(prefix, mount_prefix);
        let paths = ns.list(local_prefix).await?;

        // Restore full paths
        Ok(paths.into_iter().map(|p| {
            if mount_prefix.is_empty() {
                p
            } else {
                format!("{}{}", mount_prefix, p)
            }
        }).collect())
    }

    pub fn watch(&self, pattern: &str) -> NamespaceResult<mpsc::UnboundedReceiver<Scroll>> {
        let (_, ns) = self.route(pattern);
        ns.watch(pattern)
    }

    pub async fn close(&self) -> NamespaceResult<()> {
        for (_, ns) in &self.mounts {
            ns.close().await?;
        }
        self.default_ns.close().await?;
        Ok(())
    }
}

impl Default for WasmStore {
    fn default() -> Self {
        Self::new()
    }
}
