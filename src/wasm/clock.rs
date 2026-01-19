//! WASM Clock - UI-driven logical clock for browser clients
//!
//! Provides the same beeclock-core tick system for browser environments.
//! The clock is driven by JavaScript (requestAnimationFrame, setInterval, etc.)
//!
//! # Usage from JavaScript
//!
//! ```javascript
//! import { WasmClock } from 'beenode';
//!
//! // Create clock with BeeWallet sacred pulses
//! const clock = WasmClock.beewallet();
//!
//! // Or with default config
//! const clock = new WasmClock();
//!
//! // Get tick interval for fixed timestep
//! const tickMs = clock.intervalMs();
//!
//! // UI-driven tick loop
//! let lastTime = 0;
//! let accumulator = 0;
//!
//! function frame(timestamp) {
//!     const dt = lastTime ? timestamp - lastTime : 0;
//!     lastTime = timestamp;
//!     accumulator += dt;
//!
//!     while (accumulator >= tickMs) {
//!         accumulator -= tickMs;
//!         const pulses = clock.tick(); // Returns array of pulse names
//!         for (const pulse of pulses) {
//!             if (pulse === 'beat') bounceAnimation();
//!             if (pulse === 'glow') glowAnimation();
//!         }
//!     }
//!
//!     // Alpha for interpolation
//!     const alpha = accumulator / tickMs;
//!     render(alpha);
//!
//!     requestAnimationFrame(frame);
//! }
//!
//! requestAnimationFrame(frame);
//! ```

use beeclock_core::{Clock, TickOutcome};
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// Clock configuration for WASM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmClockConfig {
    pub interval_ms: u64,
    pub partitions: Vec<(String, u64)>,
    pub pulses: Vec<(String, u64)>,
}

impl Default for WasmClockConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            partitions: vec![
                ("sec".into(), 60),
                ("min".into(), 60),
                ("hour".into(), 24),
            ],
            pulses: vec![
                ("tick".into(), 1),
                ("second".into(), 1),
                ("minute".into(), 60),
                ("hour".into(), 3600),
            ],
        }
    }
}

impl WasmClockConfig {
    /// BeeWallet config with sacred pulses
    pub fn beewallet() -> Self {
        Self {
            interval_ms: 1000,
            partitions: vec![
                ("sec".into(), 60),
                ("min".into(), 60),
                ("hour".into(), 24),
            ],
            pulses: vec![
                ("beat".into(), 1),       // Every tick
                ("glow".into(), 21),      // 21M tribute
                ("ping".into(), 30),      // Server heartbeat
                ("sync".into(), 60),      // Wallet sync
                ("refresh".into(), 300),  // Full refresh
                ("backup".into(), 3600),  // Hourly backup
            ],
        }
    }

    /// Fast config for testing
    pub fn fast_test() -> Self {
        Self {
            interval_ms: 100,
            partitions: vec![("tick".into(), 100)],
            pulses: vec![
                ("beat".into(), 1),
                ("glow".into(), 21),
            ],
        }
    }

    fn build_clock(&self) -> Result<Clock, JsValue> {
        let mut builder = Clock::builder().least_significant_first();

        for (name, modulus) in &self.partitions {
            builder = builder.partition(name, *modulus);
        }

        for (name, period) in &self.pulses {
            builder = builder.pulse_every(name, *period);
        }

        builder.build().map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }
}

/// WASM Clock - UI-driven logical clock
///
/// Browser clients drive the clock via tick() calls from their render loop.
/// This enables frame-synced animations with fixed timestep pattern.
#[wasm_bindgen]
pub struct WasmClock {
    clock: Clock,
    interval_ms: u64,
}

#[wasm_bindgen]
impl WasmClock {
    /// Create a new clock with default configuration
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmClock, JsValue> {
        let config = WasmClockConfig::default();
        let clock = config.build_clock()?;
        Ok(Self {
            clock,
            interval_ms: config.interval_ms,
        })
    }

    /// Create a clock with BeeWallet sacred pulses
    #[wasm_bindgen]
    pub fn beewallet() -> Result<WasmClock, JsValue> {
        let config = WasmClockConfig::beewallet();
        let clock = config.build_clock()?;
        Ok(Self {
            clock,
            interval_ms: config.interval_ms,
        })
    }

    /// Create a clock with fast test config (100ms ticks)
    #[wasm_bindgen(js_name = "fastTest")]
    pub fn fast_test() -> Result<WasmClock, JsValue> {
        let config = WasmClockConfig::fast_test();
        let clock = config.build_clock()?;
        Ok(Self {
            clock,
            interval_ms: config.interval_ms,
        })
    }

    /// Create a clock from JSON config
    #[wasm_bindgen(js_name = "fromConfig")]
    pub fn from_config(config_json: &str) -> Result<WasmClock, JsValue> {
        let config: WasmClockConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
        let clock = config.build_clock()?;
        Ok(Self {
            clock,
            interval_ms: config.interval_ms,
        })
    }

    /// Tick the clock (call from render loop)
    /// Returns array of fired pulse names
    #[wasm_bindgen]
    pub fn tick(&mut self) -> js_sys::Array {
        let outcome = self.clock.tick();
        let arr = js_sys::Array::new();
        for pulse in &outcome.pulses {
            arr.push(&JsValue::from_str(&pulse.name));
        }
        arr
    }

    /// Tick and return full outcome as JSON
    #[wasm_bindgen(js_name = "tickJson")]
    pub fn tick_json(&mut self) -> String {
        let outcome = self.clock.tick();
        let result = TickResult::from(&outcome);
        serde_json::to_string(&result).unwrap_or_default()
    }

    /// Get current tick count without ticking
    #[wasm_bindgen(js_name = "currentTick")]
    pub fn current_tick(&self) -> u64 {
        self.clock.snapshot().tick
    }

    /// Get current epoch without ticking
    #[wasm_bindgen(js_name = "currentEpoch")]
    pub fn current_epoch(&self) -> u64 {
        self.clock.snapshot().epoch
    }

    /// Get tick interval in milliseconds (for fixed timestep)
    #[wasm_bindgen(js_name = "intervalMs")]
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Get snapshot as JSON
    #[wasm_bindgen]
    pub fn snapshot(&self) -> String {
        let snap = self.clock.snapshot();
        let result = SnapshotResult {
            tick: snap.tick,
            epoch: snap.epoch,
            partitions: snap.partitions.iter().map(|p| PartitionResult {
                name: p.name.clone(),
                value: p.value,
                modulus: p.modulus,
            }).collect(),
        };
        serde_json::to_string(&result).unwrap_or_default()
    }
}

impl Default for WasmClock {
    fn default() -> Self {
        Self::new().expect("default clock")
    }
}

// JSON serialization structs

#[derive(Serialize, Deserialize)]
struct TickResult {
    tick: u64,
    epoch: u64,
    pulses: Vec<String>,
    overflowed: bool,
}

impl From<&TickOutcome> for TickResult {
    fn from(outcome: &TickOutcome) -> Self {
        Self {
            tick: outcome.snapshot.tick,
            epoch: outcome.snapshot.epoch,
            pulses: outcome.pulses.iter().map(|p| p.name.clone()).collect(),
            overflowed: outcome.overflowed,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SnapshotResult {
    tick: u64,
    epoch: u64,
    partitions: Vec<PartitionResult>,
}

#[derive(Serialize, Deserialize)]
struct PartitionResult {
    name: String,
    value: u64,
    modulus: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_clock_creates() {
        let clock = WasmClock::new();
        assert!(clock.is_ok());
    }

    #[test]
    fn wasm_clock_beewallet() {
        let clock = WasmClock::beewallet().unwrap();
        assert_eq!(clock.interval_ms(), 1000);
        assert_eq!(clock.current_tick(), 0);
    }

    #[test]
    fn wasm_clock_ticks() {
        let mut clock = WasmClock::new().unwrap();
        let pulses = clock.tick();
        assert!(pulses.length() > 0); // At least "tick" pulse fires
        assert_eq!(clock.current_tick(), 1);
    }

    #[test]
    fn wasm_clock_glow_at_21() {
        let mut clock = WasmClock::beewallet().unwrap();

        // Tick 20 times - no glow
        for _ in 0..20 {
            let pulses = clock.tick();
            let has_glow = (0..pulses.length())
                .any(|i| pulses.get(i).as_string() == Some("glow".to_string()));
            assert!(!has_glow);
        }

        // Tick 21 - glow fires
        let pulses21 = clock.tick();
        let has_glow = (0..pulses21.length())
            .any(|i| pulses21.get(i).as_string() == Some("glow".to_string()));
        assert!(has_glow);
    }
}
