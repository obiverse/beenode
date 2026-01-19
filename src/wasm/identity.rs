//! WASM Identity namespace - exposes derived identity for unlocked sessions.
//!
//! Path: /system/identity

use futures::channel::mpsc;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::bech32::{ToBase32, Variant, encode};

use crate::mobi::Mobi;
use nine_s_core::prelude::Scroll;

use super::auth::WasmAuth;
use super::namespace::{NamespaceError, NamespaceResult};

const IDENTITY_PATH: &str = "/identity";
const IDENTITY_TYPE: &str = "system/identity@v1";

#[derive(Clone)]
pub struct IdentityNamespace {
    auth: WasmAuth,
    watchers: Rc<RefCell<Vec<mpsc::UnboundedSender<Scroll>>>>,
}

impl IdentityNamespace {
    pub fn new(auth: WasmAuth) -> Self {
        Self {
            auth,
            watchers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn read_identity(&self) -> NamespaceResult<Scroll> {
        let seed = self
            .auth
            .session_seed()
            .ok_or_else(|| NamespaceError::Other("node locked".into()))?;
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&seed[..32])
            .map_err(|e| NamespaceError::Other(e.to_string()))?;
        let pubkey = sk.public_key(&secp).x_only_public_key().0.serialize();
        let pubkey_hex = hex::encode(pubkey);
        let mobi = Mobi::derive(&pubkey_hex)
            .map_err(|e| NamespaceError::Other(e.to_string()))?;
        let npub = encode("npub", pubkey.to_base32(), Variant::Bech32)
            .map_err(|e| NamespaceError::Other(e.to_string()))?;

        Ok(Scroll::new(
            "/system/identity",
            json!({
                "pubkey_hex": pubkey_hex,
                "npub": npub,
                "mobi": mobi.display,
            }),
        )
        .set_type(IDENTITY_TYPE))
    }

    pub async fn read(&self, path: &str) -> NamespaceResult<Option<Scroll>> {
        Ok(Some(match path {
            IDENTITY_PATH | "" | "/" => self.read_identity()?,
            _ => return Ok(None),
        }))
    }

    pub async fn write(&self, _path: &str, _data: serde_json::Value) -> NamespaceResult<Scroll> {
        Err(NamespaceError::Other("read-only".into()))
    }

    pub async fn list(&self, _: &str) -> NamespaceResult<Vec<String>> {
        Ok(vec![IDENTITY_PATH.into()])
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
