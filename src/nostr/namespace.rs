//! NostrNamespace - Nostr protocol via 9S paths

use crate::core::paths::{nostr as paths, nostr_types as types};
use crate::identity::Identity;
use crate::node::NostrConfig;
use crate::nostr::NostrEffectHandler;
use crate::mind::EffectHandler;
use nine_s_core::prelude::*;
use serde_json::{json, Value};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::runtime::Runtime;

/// Represents an active NIP-46 connection to a remote service
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Nip46Connection {
    pubkey: String,  // server pubkey (Flutter expects 'pubkey')
    relay: String,
    app_name: Option<String>,
    app_url: Option<String>,
    connected_at: i64,
}

fn scroll(key: &str, type_: &str, data: Value) -> Scroll {
    Scroll { key: key.into(), type_: type_.into(), metadata: Metadata::default(), data }
}

pub struct NostrNamespace {
    identity: Identity,
    config: NostrConfig,
    effect: NostrEffectHandler,
    runtime: Runtime,
    connected: AtomicBool,
    /// Active NIP-46 connections (apps we've authenticated with)
    nip46_connections: RwLock<Vec<Nip46Connection>>,
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
            nip46_connections: RwLock::new(Vec::new()),
        }
    }

    fn read_connections(&self) -> Scroll {
        let connections = self.nip46_connections.read().unwrap();
        let items: Vec<Value> = connections.iter().map(|c| {
            json!({
                "pubkey": c.pubkey,
                "relay": c.relay,
                "app_name": c.app_name,
                "app_url": c.app_url,
                "connected_at": c.connected_at
            })
        }).collect();
        scroll("/nostr/connections", "nostr/connections@v1", json!({
            "connections": items,
            "count": items.len()
        }))
    }

    fn add_connection(&self, pubkey: &str, relay: &str, app_name: Option<&str>, app_url: Option<&str>) {
        let mut connections = self.nip46_connections.write().unwrap();
        // Remove existing connection to same server (update)
        connections.retain(|c| c.pubkey != pubkey);
        connections.push(Nip46Connection {
            pubkey: pubkey.to_string(),
            relay: relay.to_string(),
            app_name: app_name.map(String::from),
            app_url: app_url.map(String::from),
            connected_at: chrono::Utc::now().timestamp(),
        });
    }

    fn remove_connection(&self, pubkey: &str) -> bool {
        let mut connections = self.nip46_connections.write().unwrap();
        let len_before = connections.len();
        connections.retain(|c| c.pubkey != pubkey);
        connections.len() < len_before
    }

    fn clear_connections(&self) -> usize {
        let mut connections = self.nip46_connections.write().unwrap();
        let count = connections.len();
        connections.clear();
        count
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
        let relay = data.get("relay").and_then(|v| v.as_str());

        let id = uuid();
        let mut scroll_data = json!({
            "kind": kind,
            "content": content,
            "tags": tags,
        });
        // Include relay if specified (for NIP-46 responses to specific relays)
        if let Some(relay_url) = relay {
            scroll_data["relay"] = json!(relay_url);
        }
        let scroll_req = Scroll::new(&format!("{}/{}", paths::EXTERNAL_PUBLISH, id), scroll_data);
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
        tracing::info!("[NIP46] write_nip46_respond called");
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

        tracing::info!("[NIP46] server_pubkey: {}...", &server_pubkey_hex[..16]);
        tracing::info!("[NIP46] relay_url: {}", relay_url);
        tracing::info!("[NIP46] challenge length: {}", challenge.len());

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

        let result = self.write_publish(publish_data)?;

        // Store connection on successful response
        let app_name = data.get("app_name").and_then(|v| v.as_str());
        let app_url = data.get("app_url").and_then(|v| v.as_str());
        self.add_connection(server_pubkey_hex, relay_url, app_name, app_url);
        tracing::info!("[NIP46] Connection stored for pubkey: {}...", &server_pubkey_hex[..16]);

        Ok(result)
    }

    fn write_connections_revoke(&self, pubkey: &str) -> NineSResult<Scroll> {
        let removed = self.remove_connection(pubkey);
        Ok(scroll("/nostr/connections/revoke", "nostr/connections@v1", json!({
            "removed": removed,
            "pubkey": pubkey
        })))
    }

    fn write_connections_clear(&self) -> NineSResult<Scroll> {
        let count = self.clear_connections();
        Ok(scroll("/nostr/connections/clear", "nostr/connections@v1", json!({
            "cleared": count
        })))
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
            "/connections" => self.read_connections(),
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
            "/connections/clear" => self.write_connections_clear(),
            _ => {
                // Handle /connections/{pubkey}/revoke pattern
                if let Some(pubkey) = path.strip_prefix("/connections/")
                    .and_then(|rest| rest.strip_suffix("/revoke"))
                {
                    if !pubkey.is_empty() {
                        return self.write_connections_revoke(pubkey);
                    }
                }
                Err(NineSError::Other(format!("unknown: {}", path)))
            }
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
