# API Reference

Beenode exposes three API surfaces: HTTP (native server), Rust (library), and WASM (browser).

## HTTP API

Base URL: `http://localhost:8080` (default)

### System Endpoints

#### Health Check

```
GET /health
```

Response:
```json
{
  "status": "ok",
  "service": "beenode"
}
```

#### List Scrolls

```
GET /scrolls?prefix=/wallet
```

Response:
```json
{
  "paths": ["/wallet/status", "/wallet/balance", "/wallet/address"],
  "count": 3
}
```

#### Read Scroll

```
GET /scroll/{path}
```

Example:
```bash
curl http://localhost:8080/scroll/wallet/balance
```

Response:
```json
{
  "key": "/wallet/balance",
  "type": "wallet/balance@v1",
  "data": {
    "confirmed": 50000,
    "pending": 10000,
    "total": 60000,
    "spendable": 50000
  },
  "metadata": {
    "version": 5,
    "created_at": 1706000000,
    "updated_at": 1706001000
  }
}
```

#### Write Scroll

```
POST /scroll/{path}
Content-Type: application/json

{data}
```

Example:
```bash
curl -X POST http://localhost:8080/scroll/wallet/sync \
  -H "Content-Type: application/json" \
  -d '{}'
```

### Authentication Endpoints

#### Get Auth Status

```
GET /scroll/system/auth/status
```

Response:
```json
{
  "data": {
    "locked": true,
    "initialized": true
  }
}
```

#### Unlock

```
POST /scroll/system/auth/unlock
Content-Type: application/json

{"pin": "123456"}
```

Response:
```json
{
  "data": {
    "success": true
  }
}
```

#### Lock

```
POST /scroll/system/auth/lock
```

---

## Wallet Paths

### Read Paths

#### `/wallet/status`

Wallet initialization state.

```json
{
  "initialized": true,
  "network": "testnet"
}
```

#### `/wallet/balance`

Current balance in satoshis.

```json
{
  "confirmed": 50000,
  "pending": 10000,
  "total": 60000,
  "spendable": 50000
}
```

#### `/wallet/address`

Current receive address.

```json
{
  "address": "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx"
}
```

#### `/wallet/transactions`

Transaction history.

```json
{
  "transactions": [
    {
      "txid": "abc123...",
      "amount_sat": 50000,
      "fee_sat": 200,
      "confirmations": 6,
      "timestamp": 1706000000,
      "direction": "incoming"
    }
  ],
  "count": 1
}
```

#### `/wallet/utxos`

Unspent transaction outputs.

```json
{
  "utxos": [
    {
      "txid": "abc123...",
      "vout": 0,
      "amount_sat": 50000,
      "confirmations": 6
    }
  ],
  "total_sat": 50000
}
```

### Write Paths

#### `/wallet/sync`

Trigger blockchain sync via Electrum.

Request:
```json
{}
```

Response:
```json
{
  "queued": true,
  "effect_path": "/external/bitcoin/sync/uuid"
}
```

#### `/wallet/send`

Create and broadcast a transaction.

Request:
```json
{
  "to": "tb1q...",
  "amount_sat": 20000,
  "fee_rate": 2.0
}
```

Response:
```json
{
  "queued": true,
  "effect_path": "/external/bitcoin/send/uuid"
}
```

Check result at `/external/bitcoin/send/{uuid}/result`:
```json
{
  "success": true,
  "txid": "def456..."
}
```

#### `/wallet/fee-estimate`

Estimate fee for a transaction.

Request:
```json
{
  "to": "tb1q...",
  "amount_sat": 20000
}
```

Response:
```json
{
  "fee_sat": 300,
  "fee_rate": 2.0,
  "vsize": 150
}
```

---

## Nostr Paths

### Read Paths

#### `/nostr/status`

Relay connection state.

```json
{
  "initialized": true,
  "relays": ["wss://relay.damus.io"],
  "connected": true
}
```

#### `/nostr/pubkey`

Public key in hex format.

```json
{
  "hex": "abc123..."
}
```

#### `/nostr/mobi`

Human-readable identifier (21 digits derived from pubkey).

```json
{
  "display": "123-456-789",
  "extended": "123-456-789-012-345",
  "full": "123456789012345678901"
}
```

#### `/nostr/relays`

Configured relay URLs.

```json
{
  "urls": ["wss://relay.damus.io", "wss://nos.lol"]
}
```

### Write Paths

#### `/nostr/sign`

Sign a message or event.

Request:
```json
{
  "message": "Hello, Nostr!"
}
```

Response:
```json
{
  "signature": "sig123...",
  "pubkey": "abc123..."
}
```

#### `/nostr/connect`

Connect to configured relays.

Request:
```json
{}
```

#### `/nostr/publish`

Publish an event to relays.

Request:
```json
{
  "kind": 1,
  "content": "Hello from Beenode!",
  "tags": []
}
```

---

## Rust API

### Node Operations

```rust
use beenode::{NodeConfig, WalletConfig};

// Create node
let node = NodeConfig::new("myapp")
    .with_mnemonic("your twelve word mnemonic phrase here")
    .with_wallet(WalletConfig::testnet())
    .build()?;

// Read scroll
let balance = node.get("/wallet/balance")?;
if let Some(scroll) = balance {
    println!("Balance: {:?}", scroll.data);
}

// Write scroll
let result = node.put("/wallet/sync", json!({}))?;

// List paths
let paths = node.all("/wallet")?;
for path in paths {
    println!("{}", path);
}

// Watch for changes
let mut rx = node.on("/wallet/**")?;
while let Some(scroll) = rx.recv().await {
    println!("Changed: {}", scroll.key);
}

// Shutdown
node.close()?;
```

### Configuration

```rust
// Wallet config
let wallet = WalletConfig::new(Network::Testnet)
    .with_electrum("ssl://electrum.blockstream.info:60002");

// With RPC (for regtest)
let wallet = WalletConfig::new(Network::Regtest)
    .with_rpc("http://127.0.0.1:18443", "user", "pass");

// Nostr config
let nostr = NostrConfig::new()
    .with_relays(vec!["wss://relay.damus.io".into()]);

// Full node
let node = NodeConfig::new("myapp")
    .with_mnemonic(mnemonic)
    .with_auth_mode(AuthMode::Pin)
    .with_wallet(wallet)
    .with_nostr(nostr)
    .build()?;
```

### Patterns

```rust
use beenode::core::Pattern;

let pattern = Pattern::new("low_balance")
    .watch("/wallet/balance")
    .guard(r#""confirmed":\s*[0-9]{1,4}[^0-9]"#)
    .emit("alert/balance@v1")
    .emit_path("/alerts/${uuid}")
    .template(json!({ "level": "warning" }));

let node = NodeConfig::new("myapp")
    .with_mind(vec![pattern])
    .build()?;
```

---

## WASM API

### Initialization

```javascript
import init, { BeeNode } from './pkg/beenode.js';

// Initialize WASM module
await init();

// Create node
const bee = new BeeNode();
await bee.init();
```

### Read

```javascript
const scroll = await bee.read("/wallet/balance");
console.log(scroll.data.confirmed);
```

### Write

```javascript
const result = await bee.write("/wallet/sync", {});
console.log(result.key);
```

### List

```javascript
const paths = await bee.list("/wallet/");
paths.forEach(path => console.log(path));
```

### Watch

```javascript
// Async iterator
for await (const scroll of bee.watch("/wallet/**")) {
    console.log("Changed:", scroll.key, scroll.data);
}

// Or with callback
bee.subscribe("/wallet/**", (scroll) => {
    console.log("Changed:", scroll);
});
```

### Mind (Patterns)

```javascript
// Initialize mind with patterns
bee.initMind([
    {
        name: "low_balance_alert",
        watch: "/wallet/balance",
        guard: '"confirmed":\\s*[0-9]{1,4}[^0-9]',
        emit: "alert/balance@v1",
        emit_path: "/alerts/${uuid}",
        template: { level: "warning" }
    }
]);

// Run mind loop (call in requestAnimationFrame)
function loop() {
    bee.runMind();
    requestAnimationFrame(loop);
}
loop();
```

### Clock

```javascript
// Start clock
bee.startClock(10); // 10 Hz

// Read tick
const tick = await bee.read("/sys/clock/tick");
console.log(tick.data.tick);

// Watch pulses
for await (const pulse of bee.watch("/sys/clock/pulses/*")) {
    console.log("Pulse:", pulse.data.pulse);
}
```

### Vault (Encryption)

```javascript
// Seal data with password
const sealed = await bee.seal({
    data: { secret: "value" },
    password: "mypassword"
});

// Unseal
const unsealed = await bee.unseal(sealed, "mypassword");
console.log(unsealed.secret);
```

---

## Error Handling

### HTTP Errors

| Status | Meaning |
|--------|---------|
| 200 | Success |
| 400 | Bad request (invalid JSON, missing fields) |
| 401 | Unauthorized (node locked) |
| 404 | Scroll not found |
| 500 | Internal error |

### Rust Errors

```rust
use beenode::NineSError;

match node.get("/invalid/path") {
    Ok(Some(scroll)) => println!("{:?}", scroll),
    Ok(None) => println!("Not found"),
    Err(NineSError::Unauthorized) => println!("Node locked"),
    Err(e) => println!("Error: {}", e),
}
```

### WASM Errors

```javascript
try {
    const scroll = await bee.read("/wallet/balance");
} catch (e) {
    if (e.message.includes("locked")) {
        // Node is locked, need to unlock
    }
    console.error("Error:", e);
}
```

---

## Type Reference

### Scroll

```typescript
interface Scroll {
    key: string;          // Path
    type: string;         // Type (domain/action@version)
    data: any;            // JSON data
    metadata: {
        version: number;
        created_at: number;  // Unix timestamp
        updated_at: number;
        produced_by?: string;
    };
}
```

### Pattern

```typescript
interface Pattern {
    name: string;
    watch: string;        // Path pattern
    guard?: string;       // Regex (must match)
    veto?: string;        // Regex (must NOT match)
    extract?: string;     // Regex with captures
    emit: string;         // Output scroll type
    emit_path: string;    // Output path template
    template: any;        // Output data template
}
```

### WalletConfig

```typescript
interface WalletConfig {
    network: "bitcoin" | "testnet" | "signet" | "regtest";
    electrum_url?: string;
    rpc_url?: string;
    rpc_user?: string;
    rpc_pass?: string;
}
```

### NostrConfig

```typescript
interface NostrConfig {
    relays: string[];
    auto_connect?: boolean;
}
```
