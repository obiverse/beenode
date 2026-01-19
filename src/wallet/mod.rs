//! Wallet module - Sovereign Bitcoin via BDK 2.x
//!
//! Provides Bitcoin wallet functionality using BDK (Bitcoin Development Kit).
//! Uses beebank's keychain for seed derivation. bdk_file_store for persistence.
//!
//! # Architecture
//!
//! ```text
//! WalletNamespace (Namespace trait)
//!     │
//!     ├── read: /status, /balance, /address, /network, /transactions
//!     │
//!     └── write: /sync, /send, /fee-estimate → External paths → Effects
//!                                                                │
//!                                                                ▼
//!                                                      BitcoinEffectHandler
//!                                                                │
//!                                                                ▼
//!                                                            BdkWallet
//!                                                                │
//!                                                                ▼
//!                                                       bdk_file_store
//! ```
//!
//! # Namespace Paths
//!
//! | Path | Method | Description |
//! |------|--------|-------------|
//! | `/status` | read | `{initialized, network}` |
//! | `/balance` | read | `{confirmed, pending, total}` sats |
//! | `/address` | read | Next receive address (bech32) |
//! | `/network` | read | bitcoin/testnet/signet/regtest |
//! | `/transactions` | read | Last 50 transactions |
//! | `/sync` | write | Queue sync → `/external/bitcoin/sync/{id}` |
//! | `/send` | write | Queue send → `/external/bitcoin/send/{id}` |
//! | `/fee-estimate` | write | Estimate fee (immediate, no effect) |

mod bdk;
#[cfg(feature = "wallet")]
mod effects;
mod namespace;

pub use bdk::{TransactionDetails, WalletBalance};
#[cfg(feature = "wallet")]
pub use bdk::BdkWallet;
#[cfg(feature = "wallet")]
pub use effects::BitcoinEffectHandler;
pub use namespace::Network;
#[cfg(feature = "wallet")]
pub use namespace::WalletNamespace;
