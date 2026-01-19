//! Beenode: Universal Agentic Node. Five verbs. Effects handle side effects.
//!
//! # Architecture
//!
//! ```text
//! Node (entry point)
//!   │
//!   ├── Shell (from nine-s-shell)
//!   │     └── Kernel with mounts:
//!   │           ├── "/" → Store (encrypted persistence)
//!   │           ├── "/wallet" → WalletNamespace (BDK + file persistence)
//!   │           └── "/nostr" → NostrNamespace (keys + relay client)
//!   │
//!   ├── Identity (mnemonic → Nostr keys → Mobi)
//!   │
//!   └── Mind (optional pattern engine)
//!         └── EffectWorker (watches /external/**, executes effects)
//! ```
//!
//! # Five Verbs
//!
//! | Verb | Method | Description |
//! |------|--------|-------------|
//! | get | `node.get(path)` | Read scroll at path |
//! | put | `node.put(path, data)` | Write scroll to path |
//! | all | `node.all(prefix)` | List paths under prefix |
//! | on | `node.on(pattern)` | Watch paths matching pattern |
//! | close | `node.close()` | Shutdown node |
//!
//! # Features
//!
//! - `native` - Native platform (server, CLI, mobile FFI)
//! - `wasm` - WASM platform (browser, IndexedDB, fetch)
//! - `wallet` - Bitcoin wallet (BDK 2.x, bdk_file_store, Electrum)
//! - `nostr` - Nostr protocol (relay client, event signing)
//!
//! # Usage
//!
//! ```ignore
//! use beenode::{Node, NodeConfig, WalletConfig, NostrConfig, Network};
//!
//! let node = Node::from_config(
//!     NodeConfig::new("myapp")
//!         .with_mnemonic("abandon abandon ...")
//!         .with_wallet(WalletConfig { network: Network::Signet, .. })
//!         .with_nostr(NostrConfig::default())
//! )?;
//!
//! // Read wallet balance
//! let balance = node.get("/wallet/balance")?;
//!
//! // Get identity
//! let mobi = node.get("/nostr/mobi")?;
//! ```

// =============================================================================
// Shared modules (compile everywhere)
// =============================================================================
pub mod core;
pub mod mobi;

// Identity requires bitcoin/bip39 (native only)
#[cfg(feature = "native")]
pub mod identity;

// =============================================================================
// Native-only modules (server, CLI, filesystem, tokio)
// =============================================================================
#[cfg(feature = "native")]
pub mod auth;
#[cfg(feature = "native")]
pub mod clock;
#[cfg(feature = "native")]
pub mod logging;
#[cfg(feature = "native")]
pub mod mind;
#[cfg(feature = "native")]
pub mod namespaces;
#[cfg(feature = "native")]
pub mod node;
#[cfg(feature = "native")]
pub mod runtime;
#[cfg(feature = "native")]
pub mod server;
#[cfg(feature = "wallet")]
pub mod wallet;
#[cfg(feature = "nostr")]
pub mod nostr;

// =============================================================================
// WASM-only modules (browser, IndexedDB, wasm-bindgen)
// =============================================================================
#[cfg(feature = "wasm")]
pub mod wasm;

// =============================================================================
// Re-exports: Shared
// =============================================================================
pub use mobi::Mobi;
pub use core::pattern::{Pattern, PatternDef};
pub use nine_s_core::prelude::*;

#[cfg(feature = "native")]
pub use identity::Identity;

// =============================================================================
// Re-exports: Native
// =============================================================================
#[cfg(feature = "native")]
pub use node::{AuthMode, Node, NodeConfig};
#[cfg(feature = "native")]
pub use clock::{ClockConfig, ClockService, UiClock, start_clock, start_clock_with_config};
#[cfg(feature = "native")]
pub use mind::{EffectHandler, EffectWorker, Mind, MindConfig};
#[cfg(feature = "native")]
pub use runtime::{Shutdown, install_signal_handlers};
#[cfg(feature = "native")]
pub use server::{create_router, create_router_with_name};
#[cfg(feature = "native")]
pub use nine_s_shell::Shell;
#[cfg(feature = "native")]
pub use nine_s_store::Store;
#[cfg(feature = "wallet")]
pub use nine_s_store::{Keychain, PersistentKeychain};

#[cfg(feature = "nostr")]
pub use node::NostrConfig;
#[cfg(feature = "wallet")]
pub use node::WalletConfig;
#[cfg(feature = "wallet")]
pub use wallet::{BitcoinEffectHandler, Network, WalletNamespace};
#[cfg(feature = "nostr")]
pub use nostr::{NostrEffectHandler, RelayPool};

// =============================================================================
// Re-exports: WASM
// =============================================================================
#[cfg(feature = "wasm")]
pub use wasm::{BeeNode, WasmClock, WasmStore, WasmVault};
