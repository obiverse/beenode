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

        // Publish to all connected relays
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
