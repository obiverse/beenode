//! Node - Universal agentic node wrapping Shell
//!
//! BIP39 seed used directly for BIP84 wallet (standard derivation).
//! HKDF-derived seeds used for other protocols (Nostr, etc).

mod config;

pub use config::NodeConfig;
pub use config::AuthMode;
#[cfg(feature = "nostr")]
pub use config::NostrConfig;
#[cfg(feature = "wallet")]
pub use config::WalletConfig;

use crate::auth::PinAuth;
use crate::identity::Identity;
use crate::namespaces::auth::{AuthController, AuthNamespace, AuthStatus};
use nine_s_core::prelude::*;
use nine_s_shell::Shell;
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[cfg(feature = "wallet")]
use nine_s_store::{Keychain, PersistentKeychain, Protocol};

/// Node wraps Shell with identity, wallet, and nostr namespaces.
pub struct Node {
    inner: Arc<Mutex<NodeInner>>,
}

struct NodeInner {
    shell: Shell,
    identity: Option<Identity>,
    config: NodeConfig,
    auth: Option<PinAuth>,
    auth_initialized: bool,
    locked: bool,
    auth_mode: AuthMode,
    #[cfg(feature = "wallet")]
    wallet_mounted: bool,
}

impl Node {
    /// Create Node from config. Keychain handles seed, derives protocol seeds.
    pub fn from_config(config: NodeConfig) -> NineSResult<Self> {
        let shell = Shell::open(&config.app, &config.master_key)?;
        let auth_mode = config.auth_mode;
        let (auth, auth_initialized, locked) = match auth_mode {
            AuthMode::Pin => {
                let auth = PinAuth::load(&config.app)?;
                let auth_initialized = auth.is_initialized();
                (Some(auth), auth_initialized, auth_initialized)
            }
            AuthMode::None => (None, false, false),
        };

        let inner = Arc::new(Mutex::new(NodeInner {
            shell,
            identity: None,
            config,
            auth,
            auth_initialized,
            locked,
            auth_mode,
            #[cfg(feature = "wallet")]
            wallet_mounted: false,
        }));

        let controller = Self::auth_controller(inner.clone());
        {
            let mut guard = inner
                .lock()
                .map_err(|_| NineSError::Other("node lock".into()))?;
            guard.shell.mount("/system/auth", Box::new(AuthNamespace::new(controller)))?;
        }

        {
            let mut guard = inner
                .lock()
                .map_err(|_| NineSError::Other("node lock".into()))?;
            if !guard.locked {
                if let Some(ref mnemonic) = guard.config.mnemonic.clone() {
                    guard.initialize_with_mnemonic(mnemonic)?;
                }
            }
        }

        Ok(Self { inner })
    }

    // Five verbs
    pub fn get(&self, path: &str) -> NineSResult<Option<Scroll>> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(path)?;
        guard.shell.get(path)
    }
    pub fn put(&self, path: &str, data: Value) -> NineSResult<Scroll> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(path)?;
        guard.shell.put(path, data)
    }
    pub fn put_scroll(&self, scroll: Scroll) -> NineSResult<Scroll> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(&scroll.key)?;
        guard.shell.put_scroll(scroll)
    }
    pub fn all(&self, prefix: &str) -> NineSResult<Vec<String>> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(prefix)?;
        guard.shell.all(prefix)
    }
    pub fn on(&self, pattern: &str) -> NineSResult<nine_s_core::watch::WatchReceiver> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(pattern)?;
        guard.shell.on(pattern)
    }
    pub fn close(&self) -> NineSResult<()> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.shell.drop()
    }

    // Identity
    pub fn identity(&self) -> Option<Identity> {
        let guard = self.inner.lock().ok()?;
        if guard.locked { return None; }
        guard.identity.clone()
    }
    pub fn mobi(&self) -> Option<crate::mobi::Mobi> {
        let guard = self.inner.lock().ok()?;
        if guard.locked { return None; }
        guard.identity.as_ref().map(|i| i.mobi.clone())
    }
    pub fn pubkey_hex(&self) -> Option<String> {
        let guard = self.inner.lock().ok()?;
        if guard.locked { return None; }
        guard.identity.as_ref().map(|i| i.pubkey_hex.clone())
    }

    pub fn is_locked(&self) -> bool {
        self.inner.lock().map(|g| g.locked).unwrap_or(true)
    }

    pub fn is_initialized(&self) -> bool {
        self.inner.lock().map(|g| g.auth_initialized).unwrap_or(false)
    }

    pub fn unlock(&self, pin: &str) -> NineSResult<bool> {
        let mut guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.unlock(pin)
    }

    pub fn lock(&self) -> NineSResult<bool> {
        let mut guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.lock()
    }

    // Convenience
    pub fn exists(&self, path: &str) -> NineSResult<bool> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(path)?;
        guard.shell.exists(path)
    }
    pub fn require(&self, path: &str) -> NineSResult<Scroll> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(path)?;
        guard.shell.require(path)
    }
    pub fn count(&self, prefix: &str) -> NineSResult<usize> {
        let guard = self.inner.lock().map_err(|_| NineSError::Other("node lock".into()))?;
        guard.check_locked(prefix)?;
        guard.shell.count(prefix)
    }

    pub fn create_store(config: &NodeConfig) -> NineSResult<nine_s_store::Store> {
        nine_s_store::Store::open(&config.app, &config.master_key)
    }

    fn auth_controller(inner: Arc<Mutex<NodeInner>>) -> AuthController {
        let status_inner = inner.clone();
        let unlock_inner = inner.clone();
        let lock_inner = inner;
        AuthController::new(
            Arc::new(move || {
                let guard = status_inner
                    .lock()
                    .map_err(|_| NineSError::Other("node lock".into()))?;
                Ok(AuthStatus { locked: guard.locked, initialized: guard.auth_initialized })
            }),
            Arc::new(move |pin| {
                let mut guard = unlock_inner
                    .lock()
                    .map_err(|_| NineSError::Other("node lock".into()))?;
                guard.unlock(pin)
            }),
            Arc::new(move || {
                let mut guard = lock_inner
                    .lock()
                    .map_err(|_| NineSError::Other("node lock".into()))?;
                guard.lock()
            }),
        )
    }
}

impl NodeInner {
    fn check_locked(&self, path: &str) -> NineSResult<()> {
        if !self.locked || path.starts_with("/system/auth") {
            return Ok(());
        }
        Err(NineSError::Other("node locked".into()))
    }

    fn unlock(&mut self, pin: &str) -> NineSResult<bool> {
        if self.auth_mode == AuthMode::None {
            if self.identity.is_none() {
                if let Some(ref mnemonic) = self.config.mnemonic.clone() {
                    self.initialize_with_mnemonic(mnemonic)?;
                }
            }
            self.locked = false;
            return Ok(true);
        }
        if !self.auth_initialized {
            return Err(NineSError::Other("auth not initialized".into()));
        }
        let auth = self.auth.as_ref().ok_or_else(|| NineSError::Other("auth not available".into()))?;
        if !auth.verify_pin(pin)? {
            return Ok(false);
        }
        if self.locked {
            if self.identity.is_none() {
                let mnemonic = auth.decrypt_mnemonic(pin)?;
                self.initialize_with_mnemonic(&mnemonic)?;
            }
            self.locked = false;
        }
        Ok(true)
    }

    fn lock(&mut self) -> NineSResult<bool> {
        if self.auth_mode == AuthMode::None {
            return Ok(false);
        }
        if self.auth_initialized {
            self.locked = true;
            return Ok(true);
        }
        Ok(false)
    }

    fn initialize_with_mnemonic(&mut self, mnemonic: &str) -> NineSResult<()> {
        if self.identity.is_some() {
            return Ok(());
        }

        #[cfg(feature = "wallet")]
        let keychain = {
            let kc = PersistentKeychain::new()?;
            if !kc.has_seed()? { kc.import_seed(mnemonic)?; }
            kc
        };

        #[cfg(feature = "wallet")]
        let has_seed = keychain.has_seed()?;
        #[cfg(not(feature = "wallet"))]
        let has_seed = true;

        if has_seed {
            #[cfg(feature = "wallet")]
            { self.identity = Some(Identity::from_seed(&keychain.derive_protocol_seed(Protocol::Nostr)?)?) }
            #[cfg(not(feature = "wallet"))]
            { self.identity = Some(Identity::from_mnemonic(mnemonic)?); }
        }

        #[cfg(feature = "wallet")]
        if let Some(ref wallet_cfg) = self.config.wallet {
            if has_seed && !self.wallet_mounted {
                use crate::wallet::WalletNamespace;
                let store = Arc::new(nine_s_store::Store::open(&self.config.app, &self.config.master_key)?);

                let db_path = wallet_cfg.data_dir.clone().unwrap_or_else(|| {
                    let root = std::env::var("NINE_S_ROOT").map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from(".")));
                    root.join(&self.config.app)
                }).join("wallet.sqlite");

                if let Some(parent) = db_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| NineSError::Other(format!("mkdir: {}", e)))?;
                }

                let seed = mnemonic_to_seed(mnemonic)?;
                #[cfg(feature = "bitcoind-rpc")]
                let wallet_ns = if let Some(ref rpc) = wallet_cfg.rpc {
                    WalletNamespace::open_rpc(&seed, store, wallet_cfg.network, &db_path, &rpc.url, &rpc.user, &rpc.pass)?
                } else {
                    WalletNamespace::open(&seed, store, wallet_cfg.network, &db_path, wallet_cfg.electrum_url.as_deref())?
                };
                #[cfg(not(feature = "bitcoind-rpc"))]
                let wallet_ns = WalletNamespace::open(&seed, store, wallet_cfg.network, &db_path, wallet_cfg.electrum_url.as_deref())?;
                self.shell.mount("/wallet", Box::new(wallet_ns))?;
                self.wallet_mounted = true;
            }
        }

        #[cfg(feature = "nostr")]
        if let (Some(ref nostr_cfg), Some(ref id)) = (&self.config.nostr, &self.identity) {
            use crate::nostr::NostrNamespace;
            self.shell.mount("/nostr", Box::new(NostrNamespace::new(id.clone(), nostr_cfg.clone())))?;
        }

        Ok(())
    }
}

/// Convert BIP39 mnemonic to 64-byte seed (standard, no HKDF)
#[cfg(feature = "wallet")]
fn mnemonic_to_seed(mnemonic: &str) -> NineSResult<[u8; 64]> {
    use bip39::Mnemonic;
    let m = Mnemonic::parse(mnemonic)
        .map_err(|e| NineSError::Other(format!("Invalid mnemonic: {}", e)))?;
    Ok(m.to_seed(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use serde_json::json;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn temp_node(app: &str) -> (TempDir, Node, std::sync::MutexGuard<'static, ()>) {
        let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());
        let node = Node::from_config(NodeConfig::new(app)).expect("node");
        (dir, node, guard)
    }

    #[test]
    fn test_five_verbs() {
        let (_dir, node, _guard) = temp_node("test-five-verbs");
        let scroll = node.put("/notes/1", json!({"title": "Hello"})).unwrap();
        assert_eq!(scroll.key, "/notes/1");
        let retrieved = node.get("/notes/1").unwrap().unwrap();
        assert_eq!(retrieved.data["title"], "Hello");
        assert_eq!(node.all("/notes").unwrap(), vec!["/notes/1"]);
        node.close().unwrap();
    }

    #[test]
    fn test_with_mnemonic() {
        let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let node = Node::from_config(NodeConfig::new("test").with_mnemonic(mnemonic)).expect("node");
        assert!(node.identity().is_some());
        assert!(node.mobi().is_some());
        assert_eq!(node.mobi().unwrap().display.len(), 12);
        drop(guard);
    }
}
