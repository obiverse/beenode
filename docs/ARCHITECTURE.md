# Architecture

Beenode is built in four layers. Each layer depends only on the layer below.

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      LAYER 3: NODE                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Identity   │  │    Shell    │  │   Authentication    │  │
│  │ (mnemonic)  │  │ (mounts ns) │  │   (PIN/biometric)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    LAYER 2: NAMESPACES                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │  Wallet  │  │  Nostr   │  │   Auth   │  │   Custom   │  │
│  │  (BDK)   │  │ (relays) │  │  (PIN)   │  │    ...     │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                     LAYER 1: STORE                          │
│  ┌─────────────────────┐  ┌─────────────────────────────┐  │
│  │    Scroll Store     │  │         Keychain            │  │
│  │  (encrypted JSON)   │  │  (seed derivation, crypto)  │  │
│  └─────────────────────┘  └─────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                     LAYER 0: CLOCK                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │    Ticks    │  │   Pulses    │  │   Pulse Callbacks   │  │
│  │  (10 Hz)    │  │ (glow/beat) │  │   (pattern trigger) │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Layer 0: Clock

The clock is the heartbeat. It produces logical time as scrolls.

### Ticks

A tick is a single time unit. Default: 10 ticks per second.

```
/sys/clock/tick → { tick: 12345 }
```

### Pulses

Pulses are named intervals derived from ticks:

| Pulse | Interval | At 10Hz |
|-------|----------|---------|
| `glow` | 60 ticks | 6 seconds |
| `beat` | 60 glows | 6 minutes |
| `sync` | 24 beats | 2.4 hours |

```
/sys/clock/pulses/glow → { pulse: "glow", tick: 12300 }
/sys/clock/pulses/beat → { pulse: "beat", tick: 216000 }
```

### Clock Config

```rust
ClockConfig {
    tick_hz: 10,              // Ticks per second
    partitions: [60, 60, 24], // Hierarchical divisions
}
```

## Layer 1: Store

The store persists scrolls with encryption.

### Scroll Structure

```rust
struct Scroll {
    key: String,        // Path: "/wallet/balance"
    r#type: String,     // Type: "wallet/balance@v1"
    data: Value,        // JSON data
    metadata: Metadata, // Version, timestamps
}

struct Metadata {
    version: u64,
    created_at: i64,
    updated_at: i64,
    produced_by: Option<String>,
}
```

### Storage Backends

| Platform | Backend | Encryption |
|----------|---------|------------|
| Native | Filesystem (JSON) | AES-256-GCM |
| WASM | IndexedDB | Browser crypto |
| Mobile | Platform keychain | Hardware-backed |

### Keychain

The keychain derives keys from the master seed:

```
Mnemonic (BIP39)
    ↓
Master Seed (64 bytes)
    ├── BIP84 → Bitcoin keys (m/84'/0'/0'/...)
    └── BIP85 → Nostr keys (derived 12-word mnemonic)
```

The seed never leaves the keychain. Only derived keys are used.

## Layer 2: Namespaces

Namespaces are mounted domains that implement the five verbs.

### Mounting

```rust
let shell = Shell::new()
    .mount("/wallet", WalletNamespace::new(config))
    .mount("/nostr", NostrNamespace::new(config))
    .mount("/system/auth", AuthNamespace::new(auth));
```

### Namespace Interface

```rust
trait Namespace {
    fn get(&self, path: &str) -> Result<Option<Scroll>>;
    fn put(&self, path: &str, data: Value) -> Result<Scroll>;
    fn all(&self, prefix: &str) -> Result<Vec<String>>;
    fn on(&self, pattern: &str) -> Result<WatchReceiver>;
}
```

### Built-in Namespaces

#### Wallet Namespace (`/wallet`)

| Path | R/W | Description |
|------|-----|-------------|
| `/wallet/status` | R | Initialization state, network |
| `/wallet/balance` | R | Confirmed, pending, spendable |
| `/wallet/address` | R | Current receive address |
| `/wallet/transactions` | R | Transaction history |
| `/wallet/utxos` | R | Unspent outputs |
| `/wallet/sync` | W | Trigger Electrum sync |
| `/wallet/send` | W | Create and broadcast transaction |
| `/wallet/fee-estimate` | W | Estimate fee for amount |

#### Nostr Namespace (`/nostr`)

| Path | R/W | Description |
|------|-----|-------------|
| `/nostr/status` | R | Relay connection state |
| `/nostr/pubkey` | R | Public key (hex) |
| `/nostr/mobi` | R | Human-readable ID (21 digits) |
| `/nostr/relays` | R | Configured relay URLs |
| `/nostr/sign` | W | Sign message/event |
| `/nostr/connect` | W | Connect to relays |
| `/nostr/publish` | W | Publish event to relays |

#### Auth Namespace (`/system/auth`)

| Path | R/W | Description |
|------|-----|-------------|
| `/system/auth/status` | R | Locked state, initialized |
| `/system/auth/unlock` | W | Unlock with PIN |
| `/system/auth/lock` | W | Lock node |

## Layer 3: Node

The node is the entry point. It composes identity, shell, and authentication.

### Node Config

```rust
let node = NodeConfig::new("myapp")
    .with_mnemonic("twelve word phrase here")
    .with_auth_mode(AuthMode::Pin)
    .with_wallet(WalletConfig::testnet())
    .with_nostr(NostrConfig::default())
    .build()?;
```

### Authentication Modes

| Mode | Behavior |
|------|----------|
| `None` | No PIN, mnemonic in memory (dev only) |
| `Pin` | Mnemonic encrypted, requires unlock |

### Locked State

When locked, the node:
- Blocks all namespace operations
- Returns 401 on HTTP requests (except `/system/auth/*`)
- Requires PIN to unlock

## Effects

Effects handle side effects outside the scroll substrate.

### Effect Flow

```
1. User writes to namespace:    PUT /wallet/send { to, amount }
2. Namespace queues effect:     PUT /external/bitcoin/send/{id} { ... }
3. EffectWorker watches:        ON /external/bitcoin/**
4. Handler executes:            BitcoinEffectHandler.execute(scroll)
5. Result written:              PUT /external/bitcoin/send/{id}/result { txid }
```

### Effect Handler Interface

```rust
trait EffectHandler {
    fn watches(&self) -> &str;  // Path pattern
    async fn execute(&self, scroll: &Scroll) -> Result<Value>;
}
```

### Built-in Effects

| Handler | Watches | Actions |
|---------|---------|---------|
| `BitcoinEffectHandler` | `/external/bitcoin/**` | Sync wallet, broadcast tx |
| `NostrEffectHandler` | `/external/nostr/**` | Connect relays, publish events |

## Mind (Pattern Engine)

The Mind watches scrolls and applies pattern transformations.

### Pattern Structure

```yaml
name: alert_on_low_balance
watch: "/wallet/balance"
guard: '"confirmed":\s*[0-9]{1,4}[^0-9]'  # < 10000 sats
veto: null
extract: '"confirmed":\s*(\d+)'
emit: "alert/balance@v1"
emit_path: "/alerts/balance/${uuid}"
template:
  level: warning
  confirmed: "${1}"
  message: "Balance below 10000 sats"
```

### Pattern Fields

| Field | Purpose |
|-------|---------|
| `watch` | Path pattern to match |
| `guard` | Regex that must match data |
| `veto` | Regex that must NOT match |
| `extract` | Capture groups from data |
| `emit` | Scroll type to produce |
| `emit_path` | Output path (supports `${uuid}`, `${path.N}`) |
| `template` | Data template (supports `${N}` captures) |

### Mind Loop

```
loop {
    scroll = store.next_change()
    for pattern in patterns {
        if pattern.matches(scroll) {
            output = pattern.apply(scroll)
            store.put(output.path, output.data)
        }
    }
}
```

## Data Flow Examples

### Read Balance

```
Client: GET /scroll/wallet/balance
    ↓
Server: routes to Shell.get("/wallet/balance")
    ↓
Shell: routes to WalletNamespace.get("balance")
    ↓
WalletNamespace: calls BdkWallet.balance()
    ↓
Returns: Scroll { key: "/wallet/balance", data: { confirmed: 50000 } }
```

### Send Bitcoin

```
Client: POST /scroll/wallet/send { to: "bc1...", amount: 20000 }
    ↓
WalletNamespace.put("send", data)
    ↓
Writes: /external/bitcoin/send/{uuid} { to, amount, status: "pending" }
    ↓
EffectWorker: watches /external/bitcoin/**
    ↓
BitcoinEffectHandler.execute():
    - Build PSBT
    - Sign with keychain
    - Broadcast via Electrum
    ↓
Writes: /external/bitcoin/send/{uuid}/result { txid: "abc..." }
```

### Pattern Reaction

```
Scroll arrives: /wallet/balance { confirmed: 5000 }
    ↓
Mind checks patterns:
    - watch "/wallet/balance" ✓ matches
    - guard "confirmed < 10000" ✓ matches
    - veto null ✓ (no veto)
    ↓
Extracts: ["5000"]
    ↓
Applies template: { level: "warning", confirmed: "5000" }
    ↓
Writes: /alerts/balance/{uuid} { level: "warning", confirmed: "5000" }
```

## Platform Differences

### Native

- Full async runtime (tokio)
- HTTP server (axum)
- Filesystem persistence
- All effects available

### WASM

- Single-threaded (no tokio)
- IndexedDB persistence
- fetch for HTTP
- Wallet sync not available (no Electrum in browser)
- Mind runs in requestAnimationFrame

### Mobile (FFI)

- Platform keychain (iOS Keychain, Android Keystore)
- Platform networking
- Biometric authentication available
- All features via C FFI
