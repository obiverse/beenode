//! NostrNamespace - Nostr protocol via 9S paths

use crate::core::paths::{nostr as paths, nostr_types as types};
use crate::identity::Identity;
use crate::node::NostrConfig;
use crate::nostr::NostrEffectHandler;
use crate::mind::EffectHandler;
use nine_s_core::prelude::*;
use serde_json::{json, Value};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::runtime::Runtime;

fn scroll(key: &str, type_: &str, data: Value) -> Scroll {
    Scroll { key: key.into(), type_: type_.into(), metadata: Metadata::default(), data }
}

pub struct NostrNamespace {
    identity: Identity,
    config: NostrConfig,
    effect: NostrEffectHandler,
    runtime: Runtime,
    connected: AtomicBool,
}

impl NostrNamespace {
    pub fn new(identity: Identity, config: NostrConfig) -> Self {
        let effect = NostrEffectHandler::new(Arc::new(identity.clone()), config.relays.clone());
        let runtime = Runtime::new().expect("nostr runtime");
        Self {
            identity,
            config,
            effect,
            runtime,
            connected: AtomicBool::new(false),
        }
    }

    fn read_status(&self) -> Scroll {
        scroll("/nostr/status", types::STATUS, json!({
            "initialized": true,
            "relays": self.config.relays.len(),
            "auto_connect": self.config.auto_connect
        }))
    }

    fn read_pubkey(&self) -> Scroll {
        scroll("/nostr/pubkey", types::PUBKEY, json!({"hex": self.identity.pubkey_hex}))
    }

    fn read_mobi(&self) -> Scroll {
        scroll("/nostr/mobi", types::MOBI, json!({
            "display": self.identity.mobi.display,
            "formatted": self.identity.mobi.display_formatted(),
            "extended": self.identity.mobi.extended,
            "long": self.identity.mobi.long,
            "full": self.identity.mobi.full
        }))
    }

    fn read_relays(&self) -> Scroll {
        scroll("/nostr/relays", types::RELAYS, json!({
            "urls": self.config.relays,
            "beebase": self.config.beebase_url
        }))
    }

    fn read_beebase_status(&self) -> Scroll {
        let relay = self.config.beebase_url.clone()
            .or_else(|| self.config.relays.first().cloned());
        scroll("/nostr/beebase/status", types::STATUS, json!({
            "connected": self.connected.load(Ordering::Relaxed),
            "relay": relay
        }))
    }

    fn write_sign(&self, data: Value) -> NineSResult<Scroll> {
        let msg = data["message"].as_str().ok_or_else(|| NineSError::Other("no 'message'".into()))?;
        let tags: Vec<nostr::Tag> = Vec::new();
        let unsigned = nostr::UnsignedEvent::new(
            self.identity.nostr_keys.public_key(),
            nostr::Timestamp::now(),
            nostr::Kind::Custom(0),
            tags,
            msg.to_string()
        );
        let event = unsigned
            .sign_with_keys(&self.identity.nostr_keys)
            .map_err(|e| NineSError::Other(format!("sign: {}", e)))?;
        Ok(scroll("/nostr/sign", types::SIGNATURE, json!({
            "message": msg,
            "signature": event.sig.to_string(),
            "pubkey": self.identity.pubkey_hex,
            "event_id": event.id.to_string()
        })))
    }

    fn write_connect(&self) -> NineSResult<Scroll> {
        let id = uuid();
        let scroll_req = Scroll::new(&format!("{}/{}", paths::EXTERNAL_CONNECT, id), json!({}));
        let result = self.runtime
            .block_on(self.effect.execute(&scroll_req))
            .map_err(|e| NineSError::Other(format!("connect: {}", e)))?;
        let connected = result.get("count").and_then(|v| v.as_u64()).unwrap_or(0) > 0;
        self.connected.store(connected, Ordering::Relaxed);
        Ok(scroll("/nostr/connect", types::CONNECT, json!({
            "status": result.get("status").cloned().unwrap_or_else(|| json!("connected")),
            "relays": result.get("relays").cloned().unwrap_or_else(|| json!(self.config.relays)),
            "connected": connected
        })))
    }

    fn write_publish(&self, data: Value) -> NineSResult<Scroll> {
        let content = data["content"].as_str().ok_or_else(|| NineSError::Other("no 'content'".into()))?;
        let kind = data["kind"].as_u64().unwrap_or(1) as u16;
        let tags = data.get("tags").cloned().unwrap_or_else(|| json!([]));

        let id = uuid();
        let scroll_req = Scroll::new(&format!("{}/{}", paths::EXTERNAL_PUBLISH, id), json!({
            "kind": kind,
            "content": content,
            "tags": tags,
        }));
        let result = self.runtime
            .block_on(self.effect.execute(&scroll_req))
            .map_err(|e| NineSError::Other(format!("publish: {}", e)))?;
        Ok(scroll("/nostr/publish", types::PUBLISH, result))
    }

    fn write_beebase_connect(&self, data: Value) -> NineSResult<Scroll> {
        let relay_override = data.get("relay_url").and_then(|v| v.as_str());
        if let Some(relay) = relay_override {
            if !self.config.relays.iter().any(|r| r == relay) {
                return Err(NineSError::Other("relay not configured".into()));
            }
        }
        let result = self.write_connect()?;
        Ok(scroll("/nostr/beebase/connect", types::CONNECT, result.data))
    }

    fn write_beebase_disconnect(&self) -> NineSResult<Scroll> {
        self.connected.store(false, Ordering::Relaxed);
        Ok(scroll("/nostr/beebase/disconnect", types::STATUS, json!({"connected": false})))
    }

    fn write_nip46_respond(&self, data: Value) -> NineSResult<Scroll> {
        let server_pubkey_hex = data["server_pubkey"]
            .as_str()
            .ok_or_else(|| NineSError::Other("Missing 'server_pubkey' field".into()))?;
        let relay_url = data["relay"]
            .as_str()
            .ok_or_else(|| NineSError::Other("Missing 'relay' field".into()))?;
        let challenge = data["challenge"]
            .as_str()
            .ok_or_else(|| NineSError::Other("Missing 'challenge' field".into()))?;
        let challenge_id = data.get("challenge_id").and_then(|v| v.as_str());

        let server_pubkey = nostr::PublicKey::from_hex(server_pubkey_hex)
            .map_err(|e| NineSError::Other(format!("Invalid server pubkey: {}", e)))?;

        use nostr::secp256k1::{Message as SecpMessage, Secp256k1};
        use sha2::{Digest, Sha256};

        let secp = Secp256k1::new();
        let msg_hash = Sha256::digest(challenge.as_bytes());
        let secp_msg = SecpMessage::from_digest_slice(&msg_hash)
            .map_err(|e| NineSError::Other(format!("Hash failed: {}", e)))?;
        let sig = secp.sign_schnorr(&secp_msg, &self.identity.nostr_keys.secret_key().keypair(&secp));
        let signature_hex = hex::encode(sig.as_ref());

        let response_payload = json!({
            "id": challenge_id.unwrap_or(challenge),
            "challenge": challenge,
            "result": "ack",
            "pubkey": self.identity.pubkey_hex,
            "signature": signature_hex,
        });
        let response_json = serde_json::to_string(&response_payload)
            .map_err(|e| NineSError::Other(format!("JSON serialize failed: {}", e)))?;

        let encrypted = nostr::nips::nip44::encrypt(
            self.identity.nostr_keys.secret_key(),
            &server_pubkey,
            &response_json,
            nostr::nips::nip44::Version::V2,
        ).map_err(|e| NineSError::Other(format!("NIP-44 encryption failed: {}", e)))?;

        if !self.connected.load(Ordering::Relaxed) {
            let _ = self.write_connect();
        }

        let publish_data = json!({
            "kind": 24133,
            "content": encrypted,
            "tags": [["p", server_pubkey_hex]],
            "relay": relay_url
        });

        self.write_publish(publish_data)
    }
}

impl Namespace for NostrNamespace {
    fn read(&self, path: &str) -> NineSResult<Option<Scroll>> {
        Ok(Some(match path {
            paths::STATUS | "" | "/" => self.read_status(),
            paths::PUBKEY => self.read_pubkey(),
            paths::MOBI => self.read_mobi(),
            paths::RELAYS => self.read_relays(),
            "/beebase/status" => self.read_beebase_status(),
            _ => return Ok(None),
        }))
    }
    fn write(&self, path: &str, data: Value) -> NineSResult<Scroll> {
        match path {
            paths::SIGN => self.write_sign(data),
            paths::CONNECT => self.write_connect(),
            paths::PUBLISH => self.write_publish(data),
            "/beebase/connect" => self.write_beebase_connect(data),
            "/beebase/disconnect" => self.write_beebase_disconnect(),
            "/nip46/respond" => self.write_nip46_respond(data),
            _ => Err(NineSError::Other(format!("unknown: {}", path))),
        }
    }
    fn list(&self, _: &str) -> NineSResult<Vec<String>> {
        Ok(paths::ALL.iter().map(|s| (*s).into()).collect())
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!("{:016x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() & 0xFFFFFFFFFFFFFFFF)
}
