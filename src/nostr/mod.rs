//! Nostr - Decentralized social protocol integration
//!
//! Provides Nostr connectivity and identity for the node:
//! - Key derivation from mnemonic (via BIP85 → 12-word Nostr mnemonic)
//! - Event signing (NIP-01)
//! - Relay connections via tokio-tungstenite WebSocket
//! - Auto-reconnecting RelayPool
//! - BeeBase protocol (Kind 9000/9003 scroll transport)
//!
//! # Namespace Paths
//!
//! | Path | Method | Description |
//! |------|--------|-------------|
//! | `/status` | read | `{initialized, relays, auto_connect}` |
//! | `/pubkey` | read | `{hex}` - 32-byte x-only pubkey |
//! | `/mobi` | read | `{display, formatted, extended, long, full}` |
//! | `/relays` | read | `{urls, beebase}` - configured relays |
//! | `/sign` | write | Sign message → `{signature, event_id, pubkey}` |
//! | `/connect` | write | Queue connect → `/external/nostr/connect/{id}` |
//! | `/publish` | write | Queue publish → `/external/nostr/publish/{id}` |

mod namespace;
pub mod client;
mod effects;

pub use namespace::NostrNamespace;
pub use client::{RelayClient, RelayMessage, RelayPool, RelayState, parse_relay_message};
pub use effects::NostrEffectHandler;

use serde::{Deserialize, Serialize};

/// BeeBase 9S Protocol event kinds
pub mod kinds {
    /// Universal Scroll transport
    pub const SCROLL: u16 = 9000;
    /// Legacy request
    pub const REQUEST: u16 = 9001;
    /// Server response
    pub const RESPONSE: u16 = 9002;
    /// Watch notification
    pub const WATCH: u16 = 9003;
}

/// Nostr relay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub url: String,
    pub read: bool,
    pub write: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            url: "wss://relay.damus.io".into(),
            read: true,
            write: true,
        }
    }
}

/// Default relay list
pub fn default_relays() -> Vec<RelayConfig> {
    vec![
        RelayConfig {
            url: "wss://relay.damus.io".into(),
            read: true,
            write: true,
        },
        RelayConfig {
            url: "wss://nos.lol".into(),
            read: true,
            write: true,
        },
    ]
}

/// Event filter for subscriptions (NIP-01)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}
