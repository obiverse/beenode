//! NostrEffectHandler - Async Nostr operations for /external/nostr/**

use async_trait::async_trait;
use nine_s_core::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::identity::Identity;
use crate::mind::EffectHandler;
use crate::nostr::client::{RelayClient, RelayState};
use nostr::Tag;

/// Nostr effect handler for relay operations
pub struct NostrEffectHandler {
    identity: Arc<Identity>,
    clients: Arc<RwLock<Vec<RelayClient>>>,
    relays: Vec<String>,
}

impl NostrEffectHandler {
    pub fn new(identity: Arc<Identity>, relays: Vec<String>) -> Self {
        Self {
            identity,
            clients: Arc::new(RwLock::new(Vec::new())),
            relays,
        }
    }

    async fn do_connect(&self) -> anyhow::Result<Value> {
        let mut clients = self.clients.write().await;
        let mut connected = Vec::new();

        for url in &self.relays {
            let mut client = RelayClient::new(url.clone());
            if client.connect().await.is_ok() {
                connected.push(url.clone());
                clients.push(client);
            }
        }

        Ok(json!({
            "status": "connected",
            "relays": connected,
            "count": connected.len()
        }))
    }

    async fn do_publish(&self, scroll: &Scroll) -> anyhow::Result<Value> {
        let content = scroll.data["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("no 'content'"))?;
        let kind = scroll.data["kind"].as_u64().unwrap_or(1) as u16;
        let specific_relay = scroll.data.get("relay").and_then(|v| v.as_str());

        // Build and sign event
        let tags = parse_tags(&scroll.data);
        let unsigned = nostr::UnsignedEvent::new(
            self.identity.nostr_keys.public_key(),
            nostr::Timestamp::now(),
            nostr::Kind::Custom(kind),
            tags,
            content.to_string(),
        );
        let event = unsigned.sign_with_keys(&self.identity.nostr_keys)?;

        // If a specific relay is requested (e.g., for NIP-46 auth), publish only there
        if let Some(relay_url) = specific_relay {
            tracing::info!("[NIP46] Connecting to relay: {}", relay_url);
            let mut client = RelayClient::new(relay_url.to_string());
            match client.connect().await {
                Ok(_) => {
                    tracing::info!("[NIP46] Connected to relay, publishing event");
                    let result = client.publish(&event).await;
                    tracing::info!("[NIP46] Publish result: {:?}", result.is_ok());
                    // Client will be dropped after this scope (temporary connection)
                    return Ok(json!({
                        "status": if result.is_ok() { "published" } else { "failed" },
                        "event_id": event.id.to_string(),
                        "relays_count": if result.is_ok() { 1 } else { 0 },
                        "relay": relay_url,
                        "kind": kind
                    }));
                }
                Err(e) => {
                    tracing::error!("[NIP46] Failed to connect to relay {}: {}", relay_url, e);
                    return Err(anyhow::anyhow!("Failed to connect to relay {}: {}", relay_url, e));
                }
            }
        }

        // Publish to all connected relays (default behavior)
        let clients = self.clients.read().await;
        let mut published = 0;
        for client in clients.iter() {
            if client.state().await == RelayState::Connected {
                if client.publish(&event).await.is_ok() {
                    published += 1;
                }
            }
        }

        Ok(json!({
            "status": if published > 0 { "published" } else { "failed" },
            "event_id": event.id.to_string(),
            "relays_count": published,
            "kind": kind
        }))
    }
}

#[async_trait]
impl EffectHandler for NostrEffectHandler {
    fn watches(&self) -> &str { "/external/nostr" }

    async fn execute(&self, scroll: &Scroll) -> anyhow::Result<Value> {
        if scroll.key.contains("/connect/") {
            self.do_connect().await
        } else if scroll.key.contains("/publish/") {
            self.do_publish(scroll).await
        } else {
            Err(anyhow::anyhow!("Unknown: {}", scroll.key))
        }
    }
}

fn parse_tags(data: &Value) -> Vec<Tag> {
    let tags = data.get("tags").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    tags
        .iter()
        .filter_map(|t| {
            if let Some(arr) = t.as_array() {
                let tag_strs: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !tag_strs.is_empty() {
                    return Tag::parse(&tag_strs).ok();
                }
            }
            None
        })
        .collect()
}
