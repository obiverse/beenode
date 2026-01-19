//! Clock - Layer 0: Tick-driven logical clock
//!
//! The clock is the foundation layer that boots first. Other systems
//! (wallet, nostr, mind) can watch clock pulses and react. Apps like
//! BeeWallet can dock their entire UI and system processes to the clock.
//!
//! # Architecture
//!
//! ```text
//! App Launch
//!     │
//!     ▼
//! ClockService (Layer 0 - boots first)
//!     │
//!     ├── beeclock_core::Clock (tick engine)
//!     │     ├── partitions [sec:60, min:60, hour:24]
//!     │     └── pulses (predicate-based events)
//!     │
//!     └── Writes to 9S:
//!           ├── /sys/clock/status      (running/stopped)
//!           ├── /sys/clock/tick        (every tick)
//!           └── /sys/clock/pulses/*    (when pulses fire)
//!                   │
//!                   ▼
//!           Watchers react (UI animations, sync, backup)
//! ```
//!
//! # Integration Guide (BeeWallet / Flutter FFI)
//!
//! ## 1. Start Clock on App Launch
//!
//! In your Rust engine (e.g., `beewallet-engine/src/lib.rs`):
//!
//! ```ignore
//! use beenode::{Store, ClockConfig, start_clock_with_config, Shutdown};
//! use std::sync::Arc;
//!
//! pub struct BeeWalletEngine {
//!     store: Arc<Store>,
//!     shutdown: Shutdown,
//! }
//!
//! impl BeeWalletEngine {
//!     pub async fn start(app_name: &str) -> Self {
//!         let shutdown = Shutdown::new();
//!         let store = Arc::new(Store::open(app_name, b"").unwrap());
//!
//!         // Clock boots first with sacred pulses
//!         let _clock = start_clock_with_config(
//!             store.clone(),
//!             ClockConfig::beewallet(),
//!             shutdown.subscribe(),
//!         ).unwrap();
//!
//!         Self { store, shutdown }
//!     }
//! }
//! ```
//!
//! ## 2. Watch Pulses from Rust
//!
//! ```ignore
//! use nine_s_core::watch::WatchPattern;
//!
//! impl BeeWalletEngine {
//!     pub fn watch_glow(&self) -> WatchReceiver {
//!         let pattern = WatchPattern::parse("/sys/clock/pulses/glow").unwrap();
//!         self.store.watch(&pattern).unwrap()
//!     }
//!
//!     pub fn watch_beat(&self) -> WatchReceiver {
//!         let pattern = WatchPattern::parse("/sys/clock/pulses/beat").unwrap();
//!         self.store.watch(&pattern).unwrap()
//!     }
//!
//!     pub fn watch_sync(&self) -> WatchReceiver {
//!         let pattern = WatchPattern::parse("/sys/clock/pulses/sync").unwrap();
//!         self.store.watch(&pattern).unwrap()
//!     }
//! }
//!
//! // In a spawned task:
//! let rx = engine.watch_glow();
//! while let Ok(scroll) = rx.recv() {
//!     let tick = scroll.data["tick"].as_u64().unwrap();
//!     notify_flutter("glow", tick);
//! }
//! ```
//!
//! ## 3. Expose to Flutter via FFI
//!
//! ```ignore
//! // Callback type for pulse notifications
//! type PulseCallback = extern "C" fn(name: *const i8, tick: u64);
//! static mut PULSE_CB: Option<PulseCallback> = None;
//!
//! #[no_mangle]
//! pub extern "C" fn register_pulse_callback(cb: PulseCallback) {
//!     unsafe { PULSE_CB = Some(cb); }
//! }
//!
//! fn notify_flutter(name: &str, tick: u64) {
//!     unsafe {
//!         if let Some(cb) = PULSE_CB {
//!             let c_name = std::ffi::CString::new(name).unwrap();
//!             cb(c_name.as_ptr(), tick);
//!         }
//!     }
//! }
//! ```
//!
//! ## 4. Flutter Dart Side
//!
//! ```dart
//! class PulseBridge {
//!   final _stream = StreamController<(String, int)>.broadcast();
//!
//!   Stream<int> get beats => _stream.stream
//!       .where((e) => e.$1 == 'beat').map((e) => e.$2);
//!   Stream<int> get glows => _stream.stream
//!       .where((e) => e.$1 == 'glow').map((e) => e.$2);
//!   Stream<int> get syncs => _stream.stream
//!       .where((e) => e.$1 == 'sync').map((e) => e.$2);
//!
//!   // Called from FFI
//!   void onPulse(String name, int tick) {
//!     _stream.add((name, tick));
//!   }
//! }
//! ```
//!
//! ## 5. Sacred Pulse Widget
//!
//! ```dart
//! class SacredPulse extends StatefulWidget {
//!   final PulseBridge bridge;
//!
//!   @override
//!   _SacredPulseState createState() => _SacredPulseState();
//! }
//!
//! class _SacredPulseState extends State<SacredPulse>
//!     with TickerProviderStateMixin {
//!   late AnimationController _bounce, _glow;
//!
//!   @override
//!   void initState() {
//!     super.initState();
//!     _bounce = AnimationController(vsync: this, duration: 200.ms);
//!     _glow = AnimationController(vsync: this, duration: 500.ms);
//!
//!     // Dock to clock
//!     widget.bridge.beats.listen((_) => _bounce.forward(from: 0));
//!     widget.bridge.glows.listen((_) => _glow.forward(from: 0));
//!   }
//!
//!   @override
//!   Widget build(BuildContext context) => AnimatedBuilder(
//!     animation: Listenable.merge([_bounce, _glow]),
//!     builder: (_, child) => Transform.scale(
//!       scale: 1.0 + _bounce.value * 0.1,
//!       child: Container(
//!         decoration: BoxDecoration(boxShadow: [
//!           BoxShadow(
//!             color: Colors.orange.withOpacity(0.3 + _glow.value * 0.5),
//!             blurRadius: 20 + _glow.value * 30,
//!           )
//!         ]),
//!         child: child,
//!       ),
//!     ),
//!     child: BitcoinIcon(),
//!   );
//! }
//! ```
//!
//! # Pulse Configurations
//!
//! | Config | Pulses | Use Case |
//! |--------|--------|----------|
//! | `ClockConfig::default()` | tick, second, minute, hour | General |
//! | `ClockConfig::beewallet()` | beat, glow(21), ping(30), sync(60), refresh(300), backup(3600) | BeeWallet |
//! | `ClockConfig::fast_test()` | beat, glow(21) at 10Hz | Testing |
//!
//! # Scroll Paths
//!
//! | Path | Content |
//! |------|---------|
//! | `/sys/clock/status` | `{status, interval_ms, partitions, pulses}` |
//! | `/sys/clock/tick` | `{tick, epoch, partitions[], overflowed}` |
//! | `/sys/clock/pulses/{name}` | `{name, tick, epoch}` |
//!
//! # Sacred Numbers
//!
//! The BeeWallet config embeds Bitcoin's sacred numbers:
//! - **21** - Glow pulse (21M supply cap tribute)
//! - **60** - Sync pulse (seconds in a minute)
//! - **3600** - Backup pulse (seconds in an hour)
//!
//! # UI-Driven Clock Mode
//!
//! For frame-synced animations, use `UiClock` instead of `ClockService`.
//! The UI (Flutter) drives ticks via FFI rather than a background timer.
//!
//! ## Two Modes
//!
//! | Mode | Struct | Driver | Use Case |
//! |------|--------|--------|----------|
//! | Background | `ClockService` | tokio timer | Server, headless apps |
//! | UI-Driven | `UiClock` | Flutter render loop | Mobile, desktop with animations |
//!
//! ## Fixed Timestep Pattern (ngclock style)
//!
//! The classic game loop pattern for smooth animations:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Flutter vsync callback (60Hz)                              │
//! │                                                             │
//! │    dt = now - lastFrame                                     │
//! │    accumulator += dt                                        │
//! │                                                             │
//! │    while accumulator >= TICK_MS:     ← Fixed timestep       │
//! │        clock.tick()                  ← Discrete logic       │
//! │        accumulator -= TICK_MS                               │
//! │                                                             │
//! │    alpha = accumulator / TICK_MS     ← 0.0 to 1.0           │
//! │    render(interpolate(prev, curr, alpha))  ← Smooth visual  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Flutter Integration (UI-Driven)
//!
//! ### Rust FFI Bridge
//!
//! ```ignore
//! use beenode::{UiClock, ClockConfig};
//! use std::sync::Mutex;
//!
//! static UI_CLOCK: Mutex<Option<UiClock>> = Mutex::new(None);
//!
//! #[no_mangle]
//! pub extern "C" fn clock_init() -> u64 {
//!     let clock = UiClock::beewallet().expect("clock");
//!     let interval = clock.interval_ms();
//!     *UI_CLOCK.lock().unwrap() = Some(clock);
//!     interval
//! }
//!
//! /// Tick the clock, returns JSON of fired pulses
//! #[no_mangle]
//! pub extern "C" fn clock_tick() -> *const c_char {
//!     let mut guard = UI_CLOCK.lock().unwrap();
//!     let clock = guard.as_mut().unwrap();
//!     let outcome = clock.tick();
//!
//!     // Serialize fired pulse names
//!     let pulses: Vec<&str> = outcome.pulses.iter().map(|p| p.name.as_str()).collect();
//!     let json = serde_json::to_string(&pulses).unwrap();
//!     CString::new(json).unwrap().into_raw()
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn clock_current_tick() -> u64 {
//!     UI_CLOCK.lock().unwrap().as_ref().unwrap().current_tick()
//! }
//!
//! /// Sync clock epoch with wall time (call periodically to prevent drift)
//! #[no_mangle]
//! pub extern "C" fn clock_sync_now() {
//!     let mut guard = UI_CLOCK.lock().unwrap();
//!     if let Some(clock) = guard.as_mut() {
//!         clock.sync_epoch(std::time::SystemTime::now());
//!     }
//! }
//! ```
//!
//! ### Flutter Dart Side
//!
//! ```dart
//! class ClockBridge {
//!   late final int tickMs;
//!   int _lastFrameTime = 0;
//!   int _accumulator = 0;
//!
//!   // Animation state
//!   double _bounceValue = 0.0;
//!   double _glowValue = 0.0;
//!   double _prevBounce = 0.0;
//!   double _prevGlow = 0.0;
//!
//!   final _pulseController = StreamController<String>.broadcast();
//!   Stream<String> get pulses => _pulseController.stream;
//!
//!   void init() {
//!     tickMs = native.clock_init();
//!     // Sync with wall time on start
//!     native.clock_sync_now();
//!     // Register for vsync callbacks
//!     SchedulerBinding.instance.addPersistentFrameCallback(_onFrame);
//!   }
//!
//!   void _onFrame(Duration timestamp) {
//!     final now = timestamp.inMilliseconds;
//!     final dt = _lastFrameTime == 0 ? 0 : now - _lastFrameTime;
//!     _lastFrameTime = now;
//!
//!     _accumulator += dt;
//!
//!     // Save previous values for interpolation
//!     _prevBounce = _bounceValue;
//!     _prevGlow = _glowValue;
//!
//!     // Fixed timestep: tick when enough time accumulated
//!     while (_accumulator >= tickMs) {
//!       _accumulator -= tickMs;
//!
//!       // Tick the Rust clock
//!       final pulsesJson = native.clock_tick();
//!       final pulses = jsonDecode(pulsesJson) as List;
//!
//!       // Process pulses
//!       for (final pulse in pulses) {
//!         _pulseController.add(pulse as String);
//!         if (pulse == 'beat') _bounceValue = 1.0;
//!         if (pulse == 'glow') _glowValue = 1.0;
//!       }
//!
//!       // Decay animations
//!       _bounceValue *= 0.8;
//!       _glowValue *= 0.95;
//!     }
//!
//!     // Alpha for smooth interpolation (0.0 to 1.0)
//!     final alpha = _accumulator / tickMs;
//!
//!     // Interpolated values for smooth rendering
//!     _interpolatedBounce = lerpDouble(_prevBounce, _bounceValue, alpha)!;
//!     _interpolatedGlow = lerpDouble(_prevGlow, _glowValue, alpha)!;
//!   }
//!
//!   // Current interpolated values for rendering
//!   double _interpolatedBounce = 0.0;
//!   double _interpolatedGlow = 0.0;
//!
//!   double get bounce => _interpolatedBounce;
//!   double get glow => _interpolatedGlow;
//! }
//! ```
//!
//! ### Sacred Pulse Widget (UI-Driven)
//!
//! ```dart
//! class SacredPulse extends StatelessWidget {
//!   final ClockBridge clock;
//!
//!   @override
//!   Widget build(BuildContext context) {
//!     return AnimatedBuilder(
//!       animation: clock,  // Rebuilds on animation tick
//!       builder: (_, child) => Transform.scale(
//!         scale: 1.0 + clock.bounce * 0.15,
//!         child: Container(
//!           decoration: BoxDecoration(
//!             shape: BoxShape.circle,
//!             boxShadow: [
//!               BoxShadow(
//!                 color: Colors.orange.withOpacity(0.2 + clock.glow * 0.6),
//!                 blurRadius: 20 + clock.glow * 40,
//!                 spreadRadius: clock.glow * 10,
//!               ),
//!             ],
//!           ),
//!           child: child,
//!         ),
//!       ),
//!       child: const BitcoinIcon(size: 64),
//!     );
//!   }
//! }
//! ```
//!
//! ## Time Sync
//!
//! UI-driven clocks can drift from wall time. Sync periodically:
//!
//! ```ignore
//! // In Flutter, sync every minute
//! Timer.periodic(Duration(minutes: 1), (_) {
//!   native.clock_sync_now();
//! });
//!
//! // Or catch up if behind (e.g., app was backgrounded)
//! final behind = native.clock_ticks_behind();
//! if (behind > 10) {
//!   native.clock_catch_up(10);  // Max 10 catch-up ticks
//! }
//! ```

use beeclock_core::{Clock, TickOutcome};
use nine_s_core::prelude::*;
use nine_s_core::namespace::Namespace;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::core::paths;

/// Clock configuration
#[derive(Debug, Clone)]
pub struct ClockConfig {
    /// Tick interval in milliseconds
    pub interval_ms: u64,
    /// Partition definitions: (name, modulus)
    pub partitions: Vec<(String, u64)>,
    /// Pulse definitions: (name, period) for Every pulses
    pub pulses: Vec<(String, u64)>,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000, // 1 second default
            partitions: vec![
                ("sec".into(), 60),
                ("min".into(), 60),
                ("hour".into(), 24),
            ],
            pulses: vec![
                ("tick".into(), 1),      // Every tick
                ("second".into(), 1),    // Alias
                ("minute".into(), 60),   // Every minute
                ("hour".into(), 3600),   // Every hour
            ],
        }
    }
}

impl ClockConfig {
    /// BeeWallet clock configuration with sacred pulses
    ///
    /// The sacred pulse system:
    /// - `beat` (1): Every second - the heartbeat bounce
    /// - `glow` (21): Every 21 seconds - tribute to 21M cap
    /// - `ping` (30): Server heartbeat check
    /// - `sync` (60): Wallet sync trigger
    /// - `backup` (3600): Hourly backup
    pub fn beewallet() -> Self {
        Self {
            interval_ms: 1000,
            partitions: vec![
                ("sec".into(), 60),
                ("min".into(), 60),
                ("hour".into(), 24),
            ],
            pulses: vec![
                // Sacred pulses (UI)
                ("beat".into(), 1),       // Every tick - bounce animation
                ("glow".into(), 21),      // Every 21s - glow animation (21M tribute)

                // System pulses
                ("ping".into(), 30),      // Every 30s - server heartbeat
                ("sync".into(), 60),      // Every 60s - wallet sync
                ("refresh".into(), 300),  // Every 5min - full refresh
                ("backup".into(), 3600),  // Every hour - backup
            ],
        }
    }

    /// Minimal clock for testing (fast ticks)
    pub fn fast_test() -> Self {
        Self {
            interval_ms: 100,  // 100ms ticks (10Hz)
            partitions: vec![
                ("tick".into(), 100),
            ],
            pulses: vec![
                ("beat".into(), 1),
                ("glow".into(), 21),
            ],
        }
    }
}

impl ClockConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tick interval in milliseconds
    pub fn with_interval_ms(mut self, ms: u64) -> Self {
        self.interval_ms = ms;
        self
    }

    /// Add a pulse that fires every N ticks
    pub fn with_pulse(mut self, name: &str, period: u64) -> Self {
        self.pulses.push((name.into(), period));
        self
    }

    /// Add a partition (cascading counter digit)
    pub fn with_partition(mut self, name: &str, modulus: u64) -> Self {
        self.partitions.push((name.into(), modulus));
        self
    }

    /// Build the beeclock Clock from this config
    pub fn build_clock(&self) -> Result<Clock, beeclock_core::ClockError> {
        let mut builder = Clock::builder().least_significant_first();

        for (name, modulus) in &self.partitions {
            builder = builder.partition(name, *modulus);
        }

        for (name, period) in &self.pulses {
            builder = builder.pulse_every(name, *period);
        }

        builder.build()
    }
}

/// Tick scroll data written to /sys/clock/tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickScroll {
    pub tick: u64,
    pub epoch: u64,
    pub partitions: Vec<PartitionValue>,
    pub overflowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionValue {
    pub name: String,
    pub value: u64,
    pub modulus: u64,
}

/// Pulse scroll data written to /sys/clock/pulses/{name}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseScroll {
    pub name: String,
    pub tick: u64,
    pub epoch: u64,
}

/// Clock service - runs the tick loop and writes to 9S
pub struct ClockService {
    clock: Clock,
    config: ClockConfig,
}

// =============================================================================
// UI-Driven Clock (Flutter drives ticks via FFI)
// =============================================================================

/// UI-driven clock for frame-synced animations.
///
/// Instead of a background timer, the UI (Flutter) drives ticks.
/// This enables:
/// - Frame-synced animations (tick in vsync callback)
/// - Alpha interpolation for smooth rendering
/// - Time sync to align logical clock with wall time
///
/// # Flutter Integration
///
/// ```dart
/// class ClockBridge {
///   // FFI pointer to UiClock
///   final Pointer<UiClock> _clock;
///
///   // Track time for fixed timestep
///   int _lastFrameTime = 0;
///   int _accumulator = 0;
///   final int _tickMs = 1000;  // 1 second per tick
///
///   /// Call from Flutter's SchedulerBinding.addPersistentFrameCallback
///   void onFrame(Duration timestamp) {
///     final now = timestamp.inMilliseconds;
///     final dt = _lastFrameTime == 0 ? 0 : now - _lastFrameTime;
///     _lastFrameTime = now;
///
///     _accumulator += dt;
///
///     // Fixed timestep: tick once per second
///     while (_accumulator >= _tickMs) {
///       _accumulator -= _tickMs;
///       final pulses = nativeTick(_clock);  // FFI call
///       _handlePulses(pulses);
///     }
///
///     // Alpha for interpolation (0.0 - 1.0 progress to next tick)
///     final alpha = _accumulator / _tickMs;
///     _updateAnimations(alpha);
///   }
///
///   void _updateAnimations(double alpha) {
///     // Smooth interpolation between discrete ticks
///     bounceController.value = lerpDouble(prevBounce, currBounce, alpha)!;
///   }
/// }
/// ```
///
/// # Rust FFI
///
/// ```ignore
/// use beenode::{UiClock, ClockConfig};
/// use std::sync::Mutex;
///
/// static UI_CLOCK: Mutex<Option<UiClock>> = Mutex::new(None);
///
/// #[no_mangle]
/// pub extern "C" fn init_ui_clock() {
///     let clock = UiClock::new(ClockConfig::beewallet()).unwrap();
///     *UI_CLOCK.lock().unwrap() = Some(clock);
/// }
///
/// #[no_mangle]
/// pub extern "C" fn ui_clock_tick() -> *const c_char {
///     let mut guard = UI_CLOCK.lock().unwrap();
///     if let Some(clock) = guard.as_mut() {
///         let outcome = clock.tick();
///         // Return JSON with fired pulses
///         let json = serde_json::to_string(&outcome.pulses).unwrap();
///         // ... return as C string
///     }
///     std::ptr::null()
/// }
///
/// #[no_mangle]
/// pub extern "C" fn ui_clock_tick_with_store(store: *const Store) -> *const c_char {
///     // Tick and write to 9S store
///     let mut guard = UI_CLOCK.lock().unwrap();
///     if let Some(clock) = guard.as_mut() {
///         let store = unsafe { &*store };
///         let outcome = clock.tick_to_store(store);
///         // ...
///     }
///     std::ptr::null()
/// }
/// ```
///
/// # Time Sync
///
/// The UI clock can sync with wall time to prevent drift:
///
/// ```ignore
/// // Every minute, sync logical clock with wall time
/// if tick % 60 == 0 {
///     clock.sync_to_time(SystemTime::now());
/// }
/// ```
pub struct UiClock {
    clock: Clock,
    config: ClockConfig,
    /// Wall time of epoch (for time sync)
    epoch_time: Option<std::time::SystemTime>,
}

impl UiClock {
    /// Create a new UI-driven clock
    pub fn new(config: ClockConfig) -> Result<Self, beeclock_core::ClockError> {
        let clock = config.build_clock()?;
        Ok(Self {
            clock,
            config,
            epoch_time: None,
        })
    }

    /// Create with default config
    pub fn with_defaults() -> Result<Self, beeclock_core::ClockError> {
        Self::new(ClockConfig::default())
    }

    /// Create with BeeWallet config (sacred pulses)
    pub fn beewallet() -> Result<Self, beeclock_core::ClockError> {
        Self::new(ClockConfig::beewallet())
    }

    /// Tick the clock (called from UI render loop)
    /// Returns the tick outcome with fired pulses
    pub fn tick(&mut self) -> TickOutcome {
        self.clock.tick()
    }

    /// Tick and write to 9S store
    /// Use this when you want tick data persisted for watchers
    pub fn tick_to_store(&mut self, store: &nine_s_store::Store) -> TickOutcome {
        let outcome = self.clock.tick();
        ClockService::write_tick(store, &outcome);
        outcome
    }

    /// Get current snapshot without ticking
    pub fn snapshot(&self) -> beeclock_core::ClockSnapshot {
        self.clock.snapshot()
    }

    /// Get current tick count
    pub fn current_tick(&self) -> u64 {
        self.clock.snapshot().tick
    }

    /// Get the configured tick interval
    pub fn interval(&self) -> Duration {
        Duration::from_millis(self.config.interval_ms)
    }

    /// Get interval in milliseconds (for Flutter fixed timestep)
    pub fn interval_ms(&self) -> u64 {
        self.config.interval_ms
    }

    /// Sync clock epoch with wall time.
    /// Call this periodically to prevent drift between logical and real time.
    pub fn sync_epoch(&mut self, wall_time: std::time::SystemTime) {
        self.epoch_time = Some(wall_time);
    }

    /// Calculate expected tick count based on elapsed wall time.
    /// Returns None if sync_epoch hasn't been called.
    pub fn expected_tick(&self) -> Option<u64> {
        let epoch = self.epoch_time?;
        let elapsed = epoch.elapsed().ok()?;
        let expected = elapsed.as_millis() as u64 / self.config.interval_ms;
        Some(expected)
    }

    /// Check if clock is behind wall time (needs catch-up ticks)
    pub fn ticks_behind(&self) -> Option<i64> {
        let expected = self.expected_tick()? as i64;
        let actual = self.current_tick() as i64;
        Some(expected - actual)
    }

    /// Catch up to wall time by ticking multiple times.
    /// Returns all outcomes from catch-up ticks.
    /// Use max_ticks to prevent runaway catch-up.
    pub fn catch_up(&mut self, max_ticks: u64) -> Vec<TickOutcome> {
        let mut outcomes = Vec::new();
        let behind = self.ticks_behind().unwrap_or(0);
        let ticks_needed = behind.max(0) as u64;
        let ticks_to_run = ticks_needed.min(max_ticks);

        for _ in 0..ticks_to_run {
            outcomes.push(self.clock.tick());
        }
        outcomes
    }

    /// Catch up and write all ticks to store.
    /// Returns the last outcome (or None if no ticks needed).
    pub fn catch_up_to_store(
        &mut self,
        store: &nine_s_store::Store,
        max_ticks: u64,
    ) -> Option<TickOutcome> {
        let outcomes = self.catch_up(max_ticks);
        for outcome in &outcomes {
            ClockService::write_tick(store, outcome);
        }
        outcomes.into_iter().last()
    }
}

impl ClockService {
    /// Create a new clock service
    pub fn new(config: ClockConfig) -> Result<Self, beeclock_core::ClockError> {
        let clock = config.build_clock()?;
        Ok(Self { clock, config })
    }

    /// Create with default config
    pub fn with_defaults() -> Result<Self, beeclock_core::ClockError> {
        Self::new(ClockConfig::default())
    }

    /// Spawn the clock service as a tokio task
    /// Returns a JoinHandle that can be awaited
    pub fn spawn(
        mut self,
        store: Arc<nine_s_store::Store>,
        mut shutdown: broadcast::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        let interval = Duration::from_millis(self.config.interval_ms);

        tokio::spawn(async move {
            // Write initial status
            let _ = store.write(
                paths::clock::STATUS,
                json!({
                    "status": "running",
                    "interval_ms": self.config.interval_ms,
                    "partitions": self.config.partitions,
                    "pulses": self.config.pulses.iter().map(|(n, p)| json!({"name": n, "period": p})).collect::<Vec<_>>(),
                }),
            );

            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        // Write shutdown status
                        let _ = store.write(
                            paths::clock::STATUS,
                            json!({"status": "stopped"}),
                        );
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        let outcome = self.clock.tick();
                        Self::write_tick(&store, &outcome);
                    }
                }
            }
        })
    }

    /// Write tick outcome to 9S
    fn write_tick(store: &nine_s_store::Store, outcome: &TickOutcome) {
        // Write tick scroll
        let tick_data = TickScroll {
            tick: outcome.snapshot.tick,
            epoch: outcome.snapshot.epoch,
            partitions: outcome
                .snapshot
                .partitions
                .iter()
                .map(|p| PartitionValue {
                    name: p.name.clone(),
                    value: p.value,
                    modulus: p.modulus,
                })
                .collect(),
            overflowed: outcome.overflowed,
        };

        let scroll = Scroll::new(paths::clock::TICK, serde_json::to_value(&tick_data).unwrap_or_default())
            .set_type(paths::clock::TICK_TYPE)
            .with_metadata(Metadata::default().with_produced_by(paths::origin::CLOCK));
        let _ = store.write_scroll(scroll);

        // Write pulse scrolls for each fired pulse
        for pulse in &outcome.pulses {
            let pulse_path = format!("{}/{}", paths::clock::PULSES, pulse.name);
            let pulse_data = PulseScroll {
                name: pulse.name.clone(),
                tick: pulse.tick,
                epoch: pulse.epoch,
            };

            let scroll = Scroll::new(&pulse_path, serde_json::to_value(&pulse_data).unwrap_or_default())
                .set_type(paths::clock::PULSE_TYPE)
                .with_metadata(Metadata::default().with_produced_by(paths::origin::CLOCK));
            let _ = store.write_scroll(scroll);
        }
    }

    /// Get current snapshot without ticking (for inspection)
    pub fn snapshot(&self) -> beeclock_core::ClockSnapshot {
        self.clock.snapshot()
    }

    /// Manual tick (for testing)
    pub fn tick(&mut self) -> TickOutcome {
        self.clock.tick()
    }

    /// Get the interval
    pub fn interval(&self) -> Duration {
        Duration::from_millis(self.config.interval_ms)
    }
}

/// Start the clock service with default configuration.
/// This is the "free" clock that apps get automatically.
/// Returns a JoinHandle that can be awaited or dropped.
pub fn start_clock(
    store: Arc<nine_s_store::Store>,
    shutdown: broadcast::Receiver<()>,
) -> Result<tokio::task::JoinHandle<()>, beeclock_core::ClockError> {
    let service = ClockService::with_defaults()?;
    Ok(service.spawn(store, shutdown))
}

/// Start the clock service with custom configuration.
pub fn start_clock_with_config(
    store: Arc<nine_s_store::Store>,
    config: ClockConfig,
    shutdown: broadcast::Receiver<()>,
) -> Result<tokio::task::JoinHandle<()>, beeclock_core::ClockError> {
    let service = ClockService::new(config)?;
    Ok(service.spawn(store, shutdown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builds_clock() {
        let config = ClockConfig::default();
        let clock = config.build_clock();
        assert!(clock.is_ok());
    }

    #[test]
    fn custom_config() {
        let config = ClockConfig::new()
            .with_interval_ms(500)
            .with_partition("block", 210_000)
            .with_pulse("halving", 210_000);

        assert_eq!(config.interval_ms, 500);
        assert!(config.partitions.iter().any(|(n, _)| n == "block"));
        assert!(config.pulses.iter().any(|(n, _)| n == "halving"));
    }

    #[test]
    fn service_ticks() {
        let mut service = ClockService::with_defaults().unwrap();
        let outcome = service.tick();
        assert_eq!(outcome.snapshot.tick, 1);
    }

    // =========================================================================
    // UiClock tests
    // =========================================================================

    #[test]
    fn ui_clock_creates() {
        let clock = UiClock::with_defaults();
        assert!(clock.is_ok());
    }

    #[test]
    fn ui_clock_beewallet() {
        let clock = UiClock::beewallet().unwrap();
        assert_eq!(clock.interval_ms(), 1000);
        assert_eq!(clock.current_tick(), 0);
    }

    #[test]
    fn ui_clock_ticks() {
        let mut clock = UiClock::with_defaults().unwrap();
        let outcome = clock.tick();
        assert_eq!(outcome.snapshot.tick, 1);
        assert_eq!(clock.current_tick(), 1);

        let outcome2 = clock.tick();
        assert_eq!(outcome2.snapshot.tick, 2);
    }

    #[test]
    fn ui_clock_glow_at_21() {
        let mut clock = UiClock::beewallet().unwrap();

        // Tick 20 times - no glow
        for _ in 0..20 {
            let outcome = clock.tick();
            assert!(!outcome.pulses.iter().any(|p| p.name == "glow"));
        }

        // Tick 21 - glow fires
        let outcome21 = clock.tick();
        assert!(outcome21.pulses.iter().any(|p| p.name == "glow"));
    }

    #[test]
    fn ui_clock_time_sync() {
        use std::time::SystemTime;

        let mut clock = UiClock::with_defaults().unwrap();

        // Before sync, expected_tick returns None
        assert!(clock.expected_tick().is_none());
        assert!(clock.ticks_behind().is_none());

        // Sync to now
        clock.sync_epoch(SystemTime::now());

        // Expected tick should be 0 (just started)
        assert_eq!(clock.expected_tick(), Some(0));
        assert_eq!(clock.ticks_behind(), Some(0));
    }

    #[test]
    fn ui_clock_catch_up() {
        let mut clock = UiClock::new(ClockConfig::fast_test()).unwrap();

        // Manually tick 5 times
        for _ in 0..5 {
            clock.tick();
        }
        assert_eq!(clock.current_tick(), 5);

        // Catch up with max 3 - should tick 3 more times
        // (since we have no sync point, ticks_behind returns None, catch_up does nothing)
        let outcomes = clock.catch_up(3);
        assert!(outcomes.is_empty()); // No sync point set
    }
}
