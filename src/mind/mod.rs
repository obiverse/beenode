//! Mind: The watch loop over patterns
//!
//! Intelligence = Pattern × Iteration × Memory
//!
//! # Components
//!
//! - **Mind**: Watches all scrolls, applies patterns from `/sys/mind/patterns/*`
//! - **EffectWorker**: Watches `/external/**`, executes side effects
//! - **EffectHandler**: Trait for implementing effect handlers
//!
//! # Effect Flow
//!
//! ```text
//! write /external/bitcoin/sync/{id} → EffectWorker watches
//!                                          │
//!                                          ▼
//!                               BitcoinEffectHandler.execute()
//!                                          │
//!                                          ▼
//!                               write /external/bitcoin/sync/{id}/result
//! ```

mod effects;
mod mind;

pub use effects::{EffectHandler, EffectWorker};
pub use mind::{Mind, MindConfig};
