//! WASM Auth namespace - PIN lock/unlock status (browser edition)
//!
//! Mirrors native /system/auth paths so web clients can use the same shell verbs.

use futures::channel::mpsc;
use indexed_db_futures::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::JsValue;

use super::namespace::{NamespaceError, NamespaceResult};
use nine_s_core::prelude::Scroll;

const STATUS: &str = "/status";
const UNLOCK: &str = "/unlock";
const LOCK: &str = "/lock";

const STATUS_TYPE: &str = "system/auth/status@v1";
const UNLOCK_TYPE: &str = "system/auth/unlock@v1";
const LOCK_TYPE: &str = "system/auth/lock@v1";

const AUTH_STORE: &str = "auth";
const AUTH_KEY: &str = "state";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct PersistedAuth {
    initialized: bool,
    locked: bool,
    pin_hash: Option<String>,
}

#[derive(Clone, Debug)]
struct AuthState {
    initialized: bool,
    locked: bool,
    pin_hash: Option<String>,
    session_seed: Option<[u8; 64]>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            initialized: false,
            locked: false,
            pin_hash: None,
            session_seed: None,
        }
    }
}

#[derive(Clone)]
pub struct WasmAuth {
    state: Rc<RefCell<AuthState>>,
}

impl WasmAuth {
    pub fn new() -> Self {
        Self { state: Rc::new(RefCell::new(AuthState::default())) }
    }

    pub fn status(&self) -> (bool, bool) {
        let state = self.state.borrow();
        (state.locked, state.initialized)
    }

    pub fn unlock(&self, pin: &str) -> NamespaceResult<bool> {
        let mut state = self.state.borrow_mut();
        if state.pin_hash.is_none() {
            // Auth disabled / uninitialized: mimic AuthMode::None.
            state.locked = false;
            state.session_seed = Some(derive_seed(pin));
            return Ok(true);
        }
        if !state.initialized {
            return Err(NamespaceError::Other("auth not initialized".into()));
        }
        let hash = hash_pin(pin);
        if Some(hash) == state.pin_hash {
            state.locked = false;
            state.session_seed = Some(derive_seed(pin));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn lock(&self) -> NamespaceResult<bool> {
        let mut state = self.state.borrow_mut();
        if state.pin_hash.is_none() {
            // Auth disabled / uninitialized: mimic AuthMode::None.
            return Ok(false);
        }
        if state.initialized {
            state.locked = true;
            state.session_seed = None;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn set_pin(&self, pin: &str) {
        let mut state = self.state.borrow_mut();
        state.pin_hash = Some(hash_pin(pin));
        state.initialized = true;
        state.locked = true;
        state.session_seed = None;
    }

    pub fn snapshot(&self) -> PersistedAuth {
        let state = self.state.borrow();
        PersistedAuth {
            initialized: state.initialized,
            locked: state.locked,
            pin_hash: state.pin_hash.clone(),
        }
    }

    pub fn apply_state(&self, state: PersistedAuth) {
        let mut current = self.state.borrow_mut();
        current.initialized = state.initialized;
        current.locked = if state.initialized { true } else { state.locked };
        current.pin_hash = state.pin_hash;
        current.session_seed = None;
    }

    pub fn session_seed(&self) -> Option<[u8; 64]> {
        self.state.borrow().session_seed
    }
}

fn hash_pin(pin: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(pin.as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn derive_seed(pin: &str) -> [u8; 64] {
    use blake3::Hasher;
    let mut out = [0u8; 64];

    let mut hasher0 = Hasher::new();
    hasher0.update(pin.as_bytes());
    hasher0.update(b"seed-0");
    out[..32].copy_from_slice(hasher0.finalize().as_bytes());

    let mut hasher1 = Hasher::new();
    hasher1.update(pin.as_bytes());
    hasher1.update(b"seed-1");
    out[32..].copy_from_slice(hasher1.finalize().as_bytes());

    out
}

#[derive(Clone)]
pub struct AuthStorage {
    db_name: String,
    db: Rc<RefCell<Option<IdbDatabase>>>,
}

impl AuthStorage {
    pub async fn open(db_name: &str) -> NamespaceResult<Self> {
        let storage = Self {
            db_name: db_name.to_string(),
            db: Rc::new(RefCell::new(None)),
        };
        storage.ensure_db().await?;
        Ok(storage)
    }

    async fn ensure_db(&self) -> NamespaceResult<()> {
        if self.db.borrow().is_some() {
            return Ok(());
        }

        let mut db_req = IdbDatabase::open_u32(&self.db_name, 1)
            .map_err(|e| NamespaceError::Other(format!("auth db open: {:?}", e)))?;

        db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
            if !evt.db().object_store_names().any(|n| n == AUTH_STORE) {
                evt.db().create_object_store(AUTH_STORE)?;
            }
            Ok(())
        }));

        let db = db_req.await
            .map_err(|e| NamespaceError::Other(format!("auth db open: {:?}", e)))?;

        *self.db.borrow_mut() = Some(db);
        Ok(())
    }

    pub async fn load_state(&self) -> NamespaceResult<Option<PersistedAuth>> {
        self.ensure_db().await?;
        let value = {
            let db_ref = self.db.borrow();
            let db = db_ref.as_ref()
                .ok_or_else(|| NamespaceError::Other("auth db not open".to_string()))?;

            let tx = db.transaction_on_one_with_mode(AUTH_STORE, IdbTransactionMode::Readonly)
                .map_err(|e| NamespaceError::Other(format!("auth tx: {:?}", e)))?;

            let store = tx.object_store(AUTH_STORE)
                .map_err(|e| NamespaceError::Other(format!("auth store: {:?}", e)))?;

            store.get_owned(AUTH_KEY)
                .map_err(|e| NamespaceError::Other(format!("auth get: {:?}", e)))?
        }.await
            .map_err(|e| NamespaceError::Other(format!("auth get: {:?}", e)))?;

        match value {
            Some(js_val) => {
                let state: PersistedAuth = serde_wasm_bindgen::from_value(js_val)
                    .map_err(|e| NamespaceError::Serialization(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    pub async fn save_state(&self, state: PersistedAuth) -> NamespaceResult<()> {
        self.ensure_db().await?;
        let js_val = serde_wasm_bindgen::to_value(&state)
            .map_err(|e| NamespaceError::Serialization(e.to_string()))?;

        {
            let db_ref = self.db.borrow();
            let db = db_ref.as_ref()
                .ok_or_else(|| NamespaceError::Other("auth db not open".to_string()))?;

            let tx = db.transaction_on_one_with_mode(AUTH_STORE, IdbTransactionMode::Readwrite)
                .map_err(|e| NamespaceError::Other(format!("auth tx: {:?}", e)))?;

            let store = tx.object_store(AUTH_STORE)
                .map_err(|e| NamespaceError::Other(format!("auth store: {:?}", e)))?;

            store.put_key_val_owned(AUTH_KEY, &js_val)
                .map_err(|e| NamespaceError::Other(format!("auth put: {:?}", e)))?
        }.await
            .map_err(|e| NamespaceError::Other(format!("auth put: {:?}", e)))?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct AuthNamespace {
    auth: WasmAuth,
    storage: Option<AuthStorage>,
    watchers: Rc<RefCell<Vec<mpsc::UnboundedSender<Scroll>>>>,
}

impl AuthNamespace {
    pub fn new(auth: WasmAuth) -> Self {
        Self {
            auth,
            storage: None,
            watchers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub async fn with_storage(storage: AuthStorage, auth: WasmAuth) -> NamespaceResult<Self> {
        if let Some(state) = storage.load_state().await? {
            auth.apply_state(state);
        }
        Ok(Self {
            auth,
            storage: Some(storage),
            watchers: Rc::new(RefCell::new(Vec::new())),
        })
    }

    fn notify(&self, scroll: Scroll) {
        let watchers = self.watchers.borrow();
        for tx in watchers.iter() {
            let _ = tx.unbounded_send(scroll.clone());
        }
    }

    async fn persist(&self) -> NamespaceResult<()> {
        if let Some(storage) = &self.storage {
            storage.save_state(self.auth.snapshot()).await?;
        }
        Ok(())
    }

    fn read_status(&self) -> NamespaceResult<Scroll> {
        let (locked, initialized) = self.auth.status();
        Ok(Scroll::new("/system/auth/status", json!({
            "locked": locked,
            "initialized": initialized,
        })).set_type(STATUS_TYPE))
    }

    async fn write_unlock(&self, data: Value) -> NamespaceResult<Scroll> {
        let pin = data["pin"]
            .as_str()
            .ok_or_else(|| NamespaceError::Other("no 'pin'".into()))?;
        let success = self.auth.unlock(pin)?;
        self.persist().await?;
        let scroll = Scroll::new("/system/auth/unlock", json!({"success": success}))
            .set_type(UNLOCK_TYPE);
        self.notify(scroll.clone());
        Ok(scroll)
    }

    async fn write_lock(&self) -> NamespaceResult<Scroll> {
        let success = self.auth.lock()?;
        self.persist().await?;
        let scroll = Scroll::new("/system/auth/lock", json!({"success": success}))
            .set_type(LOCK_TYPE);
        self.notify(scroll.clone());
        Ok(scroll)
    }

    pub async fn read(&self, path: &str) -> NamespaceResult<Option<Scroll>> {
        Ok(Some(match path {
            STATUS | "" | "/" => self.read_status()?,
            _ => return Ok(None),
        }))
    }

    pub async fn write(&self, path: &str, data: Value) -> NamespaceResult<Scroll> {
        match path {
            UNLOCK => self.write_unlock(data).await,
            LOCK => self.write_lock().await,
            _ => Err(NamespaceError::Other(format!("unknown: {}", path))),
        }
    }

    pub async fn list(&self, _: &str) -> NamespaceResult<Vec<String>> {
        Ok(vec![STATUS.into(), UNLOCK.into(), LOCK.into()])
    }

    pub fn watch(&self, _pattern: &str) -> NamespaceResult<mpsc::UnboundedReceiver<Scroll>> {
        let (tx, rx) = mpsc::unbounded();
        self.watchers.borrow_mut().push(tx);
        Ok(rx)
    }

    pub async fn close(&self) -> NamespaceResult<()> {
        Ok(())
    }
}
