# Beenode Documentation

**Beenode** is a universal agentic node for sovereign computing. It implements the 9S paradigm: a substrate where all state is scrolls, all I/O is five verbs, and intelligence emerges from patterns.

## What is Beenode?

Beenode is infrastructure for building self-sovereign applications:

- **Bitcoin wallet** via BDK 2.x (Electrum/RPC sync)
- **Nostr identity** via BIP85 derivation
- **Pattern engine** (Mind) for reactive programming
- **Encrypted storage** with PIN authentication
- **Runs everywhere**: Native server, CLI, WASM/browser

## Core Concepts

| Concept | Description |
|---------|-------------|
| **Scroll** | Atomic unit of state: path + type + data + metadata |
| **Five Verbs** | `get`, `put`, `all`, `on`, `close` - the only I/O |
| **Namespace** | Domain-specific read/write (wallet, nostr, auth) |
| **Effect** | Side effects triggered by writing to `/external/**` |
| **Mind** | Pattern engine that watches scrolls and transforms |
| **Clock** | Layer 0 logical time (ticks, pulses) |

## Documentation

| Document | Description |
|----------|-------------|
| [WHITEPAPER](WHITEPAPER.md) | Vision, philosophy, and design principles |
| [QUICKSTART](QUICKSTART.md) | Get running in 5 minutes |
| [ARCHITECTURE](ARCHITECTURE.md) | System layers and data flow |
| [API](API.md) | Complete API reference (HTTP, Rust, WASM) |
| [EXAMPLES](EXAMPLES.md) | Code examples and patterns |
| [SECURITY](SECURITY.md) | Security model and cryptography |

## Quick Example

```rust
// Native Rust
let node = NodeConfig::new("myapp")
    .with_mnemonic("your twelve word mnemonic phrase here")
    .with_wallet(WalletConfig::testnet())
    .build()?;

// Read balance
let balance = node.get("/wallet/balance")?;

// Send Bitcoin
node.put("/wallet/send", json!({
    "to": "tb1q...",
    "amount_sat": 50000
}))?;
```

```javascript
// Browser WASM
const bee = new BeeNode();
await bee.init();

const balance = await bee.read("/wallet/balance");
await bee.write("/wallet/send", { to: "tb1q...", amount_sat: 50000 });
```

## License

MIT OR Apache-2.0
