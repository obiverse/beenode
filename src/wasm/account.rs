//! WASM Account namespace - initialize local auth state (browser edition)
//!
//! Provides /system/account/init to set the PIN and lock state.

use futures::channel::mpsc;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;

use super::auth::{AuthStorage, WasmAuth};
use super::namespace::{NamespaceError, NamespaceResult};
use nine_s_core::prelude::Scroll;

const INIT: &str = "/init";
const INIT_TYPE: &str = "system/account/init@v1";

#[derive(Clone)]
pub struct AccountNamespace {
    auth: WasmAuth,
    storage: Option<AuthStorage>,
    watchers: Rc<RefCell<Vec<mpsc::UnboundedSender<Scroll>>>>,
}

impl AccountNamespace {
    pub fn new(auth: WasmAuth, storage: Option<AuthStorage>) -> Self {
        Self {
            auth,
            storage,
            watchers: Rc::new(RefCell::new(Vec::new())),
        }
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

    async fn write_init(&self, data: Value) -> NamespaceResult<Scroll> {
        let pin = data["pin"]
            .as_str()
            .ok_or_else(|| NamespaceError::Other("no 'pin'".into()))?;
        self.auth.set_pin(pin);
        self.persist().await?;
        let scroll = Scroll::new("/system/account/init", json!({"success": true}))
            .set_type(INIT_TYPE);
        self.notify(scroll.clone());
        Ok(scroll)
    }

    pub async fn read(&self, _path: &str) -> NamespaceResult<Option<Scroll>> {
        Ok(None)
    }

    pub async fn write(&self, path: &str, data: Value) -> NamespaceResult<Scroll> {
        match path {
            INIT => self.write_init(data).await,
            _ => Err(NamespaceError::Other(format!("unknown: {}", path))),
        }
    }

    pub async fn list(&self, _: &str) -> NamespaceResult<Vec<String>> {
        Ok(vec![INIT.into()])
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
