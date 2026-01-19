//! WASM module: Browser-native 9S node
//!
//! Provides BeeNode and WasmStore for browser environments with:
//! - IndexedDB persistence
//! - Memory namespace for fast cache
//! - Pattern matching (Mind)
//! - Clock (Layer 0) - tick-driven logical clock
//! - JS bindings via wasm-bindgen
//!
//! Architecture:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           BeeNode (JS API)              │
//! │  read, write, list, watch, close        │
//! │  initMind, runMind                      │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │           WasmClock (Layer 0)           │
//! │  tick(), pulses, fixed timestep         │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │              Mind (Runtime)             │
//! │  watch loop + pattern application       │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │         Pattern (Pure Engine)           │
//! │  x/g/v/then - no I/O, portable          │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │         WasmStore (Substrate)           │
//! │  IndexedDB / Memory namespaces          │
//! └─────────────────────────────────────────┘
//! ```

mod clock;
mod namespace;
mod store;
mod mind;
mod node;
mod auth;
mod account;
#[cfg(feature = "bitcoin")]
mod identity;
mod vault;

pub use clock::WasmClock;
pub use namespace::{MemoryNamespace, IndexedDbNamespace, Namespace, NamespaceError, NamespaceResult};
pub use store::WasmStore;
pub use mind::Mind;
pub use node::BeeNode;
pub use vault::WasmVault;

use wasm_bindgen::prelude::*;

/// Initialize WASM module (called automatically on load)
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Log to browser console
pub fn console_log(s: &str) {
    web_sys::console::log_1(&JsValue::from_str(s));
}

macro_rules! log {
    ($($t:tt)*) => {
        crate::wasm::console_log(&format!($($t)*))
    }
}

pub(crate) use log;
