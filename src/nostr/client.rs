//! Nostr relay client - tokio-tungstenite WebSocket
//!
//! Minimal implementation for connecting to relays and publishing events.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Relay connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayState {
    Disconnected,
    Connecting,
    Connected,
}

/// Nostr relay client
pub struct RelayClient {
    url: String,
    state: Arc<RwLock<RelayState>>,
    tx: Option<mpsc::Sender<String>>,
}

impl RelayClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            state: Arc::new(RwLock::new(RelayState::Disconnected)),
            tx: None,
        }
    }

    pub async fn state(&self) -> RelayState {
        *self.state.read().await
    }

    /// Connect to relay
    pub async fn connect(&mut self) -> anyhow::Result<mpsc::Receiver<String>> {
        *self.state.write().await = RelayState::Connecting;

        let (ws, _) = connect_async(&self.url).await?;
        let (mut write, mut read) = ws.split();

        // Channel for outgoing messages
        let (out_tx, mut out_rx) = mpsc::channel::<String>(32);
        self.tx = Some(out_tx);

        // Channel for incoming messages
        let (in_tx, in_rx) = mpsc::channel::<String>(64);

        let state = self.state.clone();
        *state.write().await = RelayState::Connected;

        // Spawn writer task
        let state_w = state.clone();
        tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if write.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
            *state_w.write().await = RelayState::Disconnected;
        });

        // Spawn reader task
        let state_r = state.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let Message::Text(txt) = msg {
                    if in_tx.send(txt).await.is_err() {
                        break;
                    }
                }
            }
            *state_r.write().await = RelayState::Disconnected;
        });

        Ok(in_rx)
    }

    /// Send raw message
    pub async fn send(&self, msg: &str) -> anyhow::Result<()> {
        if let Some(tx) = &self.tx {
            tx.send(msg.to_string()).await?;
            Ok(())
        } else {
            anyhow::bail!("Not connected")
        }
    }

    /// Publish event (NIP-01)
    pub async fn publish(&self, event: &nostr::Event) -> anyhow::Result<()> {
        let msg = json!(["EVENT", event]).to_string();
        self.send(&msg).await
    }

    /// Subscribe (NIP-01)
    pub async fn subscribe(&self, id: &str, filters: Vec<Value>) -> anyhow::Result<()> {
        let mut msg = vec![json!("REQ"), json!(id)];
        msg.extend(filters);
        self.send(&Value::Array(msg).to_string()).await
    }

    /// Unsubscribe (NIP-01)
    pub async fn unsubscribe(&self, id: &str) -> anyhow::Result<()> {
        let msg = json!(["CLOSE", id]).to_string();
        self.send(&msg).await
    }
}

/// Parse relay message
pub fn parse_relay_message(msg: &str) -> Option<RelayMessage> {
    let arr: Vec<Value> = serde_json::from_str(msg).ok()?;
    let cmd = arr.first()?.as_str()?;
    match cmd {
        "EVENT" => {
            let sub_id = arr.get(1)?.as_str()?.to_string();
            let event: nostr::Event = serde_json::from_value(arr.get(2)?.clone()).ok()?;
            Some(RelayMessage::Event { sub_id, event })
        }
        "OK" => {
            let event_id = arr.get(1)?.as_str()?.to_string();
            let accepted = arr.get(2)?.as_bool()?;
            let message = arr.get(3).and_then(|v| v.as_str()).map(String::from);
            Some(RelayMessage::Ok { event_id, accepted, message })
        }
        "EOSE" => {
            let sub_id = arr.get(1)?.as_str()?.to_string();
            Some(RelayMessage::Eose { sub_id })
        }
        "NOTICE" => {
            let message = arr.get(1)?.as_str()?.to_string();
            Some(RelayMessage::Notice { message })
        }
        _ => None,
    }
}

/// Relay message types
#[derive(Debug)]
pub enum RelayMessage {
    Event { sub_id: String, event: nostr::Event },
    Ok { event_id: String, accepted: bool, message: Option<String> },
    Eose { sub_id: String },
    Notice { message: String },
}

/// Auto-reconnecting relay pool
pub struct RelayPool {
    relays: Arc<RwLock<Vec<(String, RelayClient)>>>,
    shutdown: Arc<RwLock<bool>>,
}

impl RelayPool {
    pub fn new(urls: Vec<String>) -> Self {
        let relays = urls.into_iter().map(|u| (u.clone(), RelayClient::new(u))).collect();
        Self {
            relays: Arc::new(RwLock::new(relays)),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Start pool with automatic reconnection
    pub async fn start(&self) {
        let relays = self.relays.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            loop {
                if *shutdown.read().await { break; }

                let mut clients = relays.write().await;
                for (url, client) in clients.iter_mut() {
                    if client.state().await == RelayState::Disconnected {
                        tracing::info!("Reconnecting to {}", url);
                        let _ = client.connect().await;
                    }
                }
                drop(clients);

                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });
    }

    /// Publish to all connected relays
    pub async fn publish(&self, event: &nostr::Event) -> usize {
        let clients = self.relays.read().await;
        let mut count = 0;
        for (_, client) in clients.iter() {
            if client.state().await == RelayState::Connected {
                if client.publish(event).await.is_ok() { count += 1; }
            }
        }
        count
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) {
        *self.shutdown.write().await = true;
    }
}
