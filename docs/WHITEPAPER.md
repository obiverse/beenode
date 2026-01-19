# Beenode Whitepaper

## The Problem

Modern applications fragment user data across services, each with its own API, authentication, and storage. Users don't own their data. Developers rebuild the same primitives repeatedly. Intelligence is trapped in silos.

## The Vision

**Beenode** implements a universal computing substrate where:

1. **All state is scrolls** - Immutable, versioned, encrypted
2. **All I/O is five verbs** - `get`, `put`, `all`, `on`, `close`
3. **Intelligence emerges from patterns** - Watch, match, transform
4. **Programs are agents** - All code reads/writes the same substrate

This is **9S** (Nine Scrolls) - a filesystem for the sovereign age.

## Core Principles

### 1. Everything is a Scroll

A scroll is the atomic unit of state:

```
Scroll {
    key: "/wallet/balance",
    type: "wallet/balance@v1",
    data: { confirmed: 50000, pending: 10000 },
    metadata: { version: 3, created_at: 1706000000 }
}
```

- **Path** = identity (no UUIDs, no foreign keys)
- **Type** = schema (domain/action@version)
- **Data** = JSON value
- **Metadata** = version, timestamps, provenance

### 2. Five Verbs, No Exceptions

| Verb | Signature | Purpose |
|------|-----------|---------|
| `get` | `(path) → Scroll?` | Read single scroll |
| `put` | `(path, data) → Scroll` | Write scroll |
| `all` | `(prefix) → [paths]` | List paths under prefix |
| `on` | `(pattern) → Stream` | Watch paths matching pattern |
| `close` | `() → ()` | Shutdown cleanly |

Every namespace, every protocol, every effect - all use these five verbs. There is no sixth.

### 3. Namespaces as Composition

Namespaces are mounted domains that implement the five verbs:

```
/wallet/*     → WalletNamespace (BDK Bitcoin)
/nostr/*      → NostrNamespace (keys, relays)
/system/*     → SystemNamespace (auth, config)
/external/*   → Effects (side effects)
```

Reading `/wallet/balance` routes to `WalletNamespace.get("balance")`. The namespace translates scroll operations to domain-specific actions.

### 4. Effects for Side Effects

Side effects (network, hardware, external APIs) are triggered by writing to `/external/**`:

```
Write: /wallet/send { to: "bc1...", amount: 50000 }
    ↓
Namespace writes: /external/bitcoin/send/{id} { ... }
    ↓
EffectHandler executes: broadcast transaction
    ↓
Result written: /external/bitcoin/send/{id}/result { txid: "..." }
```

Effects are:
- **Stateless** - Pure functions from scroll to scroll
- **Isolated** - Cannot read/write arbitrary paths
- **Recoverable** - Pending requests survive restarts

### 5. Mind: Pattern-Driven Intelligence

The Mind is a pattern engine that watches scrolls and applies transformations:

```yaml
patterns:
  - name: low_balance_alert
    watch: "/wallet/balance"
    guard: '"confirmed":\\s*[0-9]{1,4}[^0-9]'  # < 10000 sats
    emit: "alert/low_balance@v1"
    emit_path: "/alerts/balance/${uuid}"
    template: { level: "warning", message: "Balance low" }
```

When a scroll matches:
1. **watch** - Path pattern matches
2. **guard** - Regex matches data (optional)
3. **veto** - Regex does NOT match (optional)
4. **extract** - Capture groups from data

Then:
- **emit** - Produce scroll of this type
- **emit_path** - At this path (with substitutions)
- **template** - With this data (with substitutions)

**Intelligence = Pattern × Iteration × Memory**

### 6. Layer 0 Clock

Time is a scroll. The clock produces ticks at a fixed rate, and pulses at intervals:

```
/sys/clock/tick     → { tick: 12345 }
/sys/clock/pulses/glow  → every 60 ticks (6 seconds at 10Hz)
/sys/clock/pulses/beat  → every 60 glows (6 minutes)
/sys/clock/pulses/sync  → every 24 beats (2.4 hours)
```

Pulses trigger patterns. A pattern watching `/sys/clock/pulses/sync` runs on schedule.

### 7. Programs are 9S Agents

There is no distinction between "application" and "infrastructure":

- Claude (AI) reads/writes scrolls
- Flutter (UI) reads/writes scrolls
- Cron (scheduler) reads/writes scrolls
- Beenode itself reads/writes scrolls

All agents share the same substrate. Communication is scroll exchange.

## Architecture

```
┌─────────────────────────────────────────┐
│           Layer 3: Node                 │
│  (Identity, Shell, Authentication)      │
├─────────────────────────────────────────┤
│         Layer 2: Namespaces             │
│  (Wallet, Nostr, Auth, Custom)          │
├─────────────────────────────────────────┤
│          Layer 1: Store                 │
│  (Encrypted persistence, Keychain)      │
├─────────────────────────────────────────┤
│          Layer 0: Clock                 │
│  (Ticks, Pulses, Logical time)          │
└─────────────────────────────────────────┘
```

Each layer depends only on the layer below. No upward dependencies.

## Security Model

1. **Mnemonic is root** - 12/24-word BIP39 phrase generates all keys
2. **PIN encryption** - Mnemonic encrypted with Argon2id + AES-256-GCM
3. **Key derivation** - BIP84 for Bitcoin, BIP85 for Nostr
4. **Locked by default** - Node requires unlock before operations
5. **No key export** - Seed stays in keychain, only derived keys used

## Platform Support

| Platform | Storage | Networking | Features |
|----------|---------|------------|----------|
| Native (server/CLI) | Filesystem | tokio/axum | Full |
| WASM (browser) | IndexedDB | fetch | No wallet sync |
| Mobile (FFI) | Platform keychain | Platform network | Full |

## The Treatise

### On Simplicity

> "The right amount of complexity is the minimum needed for the current task."

9S has five verbs because five is enough. Adding a sixth would fragment the abstraction. Removing one would cripple functionality.

### On Sovereignty

> "Sovereign computing means the user controls the substrate."

Beenode runs locally. No server required. Data encrypted at rest. Keys never leave the device. The user is the authority.

### On Composition

> "Small things that compose beat large things that don't."

Scrolls compose via paths. Namespaces compose via mounting. Patterns compose via chaining. Effects compose via queuing. Each primitive is simple; together they express any computation.

### On Intelligence

> "Intelligence = Pattern × Iteration × Memory"

The Mind watches. Patterns match. Scrolls transform. Memory persists. Iteration continues. Intelligence emerges not from complexity but from simplicity applied repeatedly.

## Conclusion

Beenode is not an application. It is a substrate for applications. It is not a wallet. It is a namespace that implements wallet operations. It is not a social network. It is a namespace that implements Nostr.

The future is sovereign. The substrate is 9S. The node is Beenode.
