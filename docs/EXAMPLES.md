# Examples

Practical code examples for common Beenode tasks.

## Basic Operations

### Read and Display Balance (Rust)

```rust
use beenode::{NodeConfig, WalletConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node = NodeConfig::new("wallet-viewer")
        .with_mnemonic("your twelve word mnemonic phrase here")
        .with_wallet(WalletConfig::testnet())
        .build()?;

    // Sync first
    node.put("/wallet/sync", serde_json::json!({}))?;

    // Wait a moment for sync
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Read balance
    if let Some(scroll) = node.get("/wallet/balance")? {
        let confirmed = scroll.data["confirmed"].as_u64().unwrap_or(0);
        let pending = scroll.data["pending"].as_u64().unwrap_or(0);

        println!("Confirmed: {} sats", confirmed);
        println!("Pending:   {} sats", pending);
        println!("Total:     {} sats", confirmed + pending);
    }

    Ok(())
}
```

### Read Balance (HTTP/curl)

```bash
# Sync wallet
curl -X POST http://localhost:8080/scroll/wallet/sync -d '{}'

# Wait for sync, then read balance
curl http://localhost:8080/scroll/wallet/balance | jq '.data'
```

### Read Balance (JavaScript/WASM)

```javascript
import init, { BeeNode } from './pkg/beenode.js';

async function main() {
    await init();
    const bee = new BeeNode();
    await bee.init();

    const balance = await bee.read("/wallet/balance");
    console.log(`Balance: ${balance.data.confirmed} sats`);
}

main();
```

---

## Sending Bitcoin

### Send Transaction (Rust)

```rust
use beenode::{NodeConfig, WalletConfig};
use serde_json::json;

async fn send_bitcoin(to: &str, amount_sat: u64) -> Result<String, Box<dyn std::error::Error>> {
    let node = NodeConfig::new("sender")
        .with_mnemonic("your mnemonic here")
        .with_wallet(WalletConfig::testnet())
        .build()?;

    // Ensure wallet is synced
    node.put("/wallet/sync", json!({}))?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Check balance
    let balance = node.get("/wallet/balance")?.unwrap();
    let spendable = balance.data["spendable"].as_u64().unwrap_or(0);

    if spendable < amount_sat + 1000 {  // amount + estimated fee
        return Err("Insufficient balance".into());
    }

    // Send
    let result = node.put("/wallet/send", json!({
        "to": to,
        "amount_sat": amount_sat,
        "fee_rate": 2.0  // sat/vbyte
    }))?;

    // Get the effect path to check result
    let effect_path = result.data["effect_path"].as_str().unwrap();

    // Poll for result
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        if let Some(result) = node.get(&format!("{}/result", effect_path))? {
            if result.data["success"].as_bool() == Some(true) {
                return Ok(result.data["txid"].as_str().unwrap().to_string());
            }
        }
    }

    Err("Transaction timed out".into())
}
```

### Send Transaction (HTTP)

```bash
# Send 20000 sats
curl -X POST http://localhost:8080/scroll/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "to": "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
    "amount_sat": 20000,
    "fee_rate": 2.0
  }'

# Response includes effect_path
# {"queued":true,"effect_path":"/external/bitcoin/send/abc123"}

# Check result
curl http://localhost:8080/scroll/external/bitcoin/send/abc123/result
```

---

## Watching for Changes

### Watch Balance Changes (Rust)

```rust
use beenode::NodeConfig;

async fn watch_balance() -> Result<(), Box<dyn std::error::Error>> {
    let node = NodeConfig::new("watcher")
        .with_mnemonic("your mnemonic here")
        .with_wallet(WalletConfig::testnet())
        .build()?;

    let mut rx = node.on("/wallet/balance")?;

    println!("Watching for balance changes...");

    while let Some(scroll) = rx.recv().await {
        let confirmed = scroll.data["confirmed"].as_u64().unwrap_or(0);
        println!("Balance changed: {} sats", confirmed);
    }

    Ok(())
}
```

### Watch All Wallet Events (JavaScript)

```javascript
const bee = new BeeNode();
await bee.init();

// Watch all wallet paths
for await (const scroll of bee.watch("/wallet/**")) {
    console.log(`${scroll.key} changed:`, scroll.data);
}
```

---

## Pattern-Based Alerts

### Low Balance Alert Pattern

```rust
use beenode::{NodeConfig, Pattern};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let low_balance_pattern = Pattern::new("low_balance_alert")
        .watch("/wallet/balance")
        .guard(r#""confirmed":\s*[0-9]{1,4}[^0-9]"#)  // < 10000 sats
        .emit("alert/low_balance@v1")
        .emit_path("/alerts/balance/${uuid}")
        .template(json!({
            "level": "warning",
            "message": "Balance is below 10000 sats"
        }));

    let node = NodeConfig::new("alerter")
        .with_mnemonic("your mnemonic here")
        .with_wallet(WalletConfig::testnet())
        .with_mind(vec![low_balance_pattern])
        .build()?;

    // Watch for alerts
    let mut rx = node.on("/alerts/**")?;

    while let Some(alert) = rx.recv().await {
        println!("ALERT: {}", alert.data["message"]);
    }

    Ok(())
}
```

### Transaction Notification Pattern

```rust
let tx_pattern = Pattern::new("new_tx_notification")
    .watch("/wallet/transactions")
    .extract(r#""txid":\s*"([^"]+)"#)
    .emit("notification/tx@v1")
    .emit_path("/notifications/${uuid}")
    .template(json!({
        "title": "New Transaction",
        "txid": "${1}"
    }));
```

### Periodic Sync Pattern

```rust
// Sync wallet every 6 minutes (on "beat" pulse)
let sync_pattern = Pattern::new("periodic_sync")
    .watch("/sys/clock/pulses/beat")
    .emit("internal/sync_request@v1")
    .emit_path("/wallet/sync")
    .template(json!({}));
```

---

## Authentication

### PIN Setup and Unlock (Rust)

```rust
use beenode::{NodeConfig, AuthMode};

async fn setup_pin_auth() -> Result<(), Box<dyn std::error::Error>> {
    // Create node with PIN auth
    let node = NodeConfig::new("secure-wallet")
        .with_mnemonic("your mnemonic here")
        .with_auth_mode(AuthMode::Pin)
        .build()?;

    // Node starts locked
    let status = node.get("/system/auth/status")?.unwrap();
    assert!(status.data["locked"].as_bool().unwrap());

    // Unlock with PIN
    node.put("/system/auth/unlock", json!({ "pin": "123456" }))?;

    // Now we can use the wallet
    let balance = node.get("/wallet/balance")?;
    println!("Balance: {:?}", balance);

    // Lock when done
    node.put("/system/auth/lock", json!({}))?;

    Ok(())
}
```

### PIN Auth (HTTP)

```bash
# Check status
curl http://localhost:8080/scroll/system/auth/status
# {"data":{"locked":true,"initialized":true}}

# Unlock
curl -X POST http://localhost:8080/scroll/system/auth/unlock \
  -H "Content-Type: application/json" \
  -d '{"pin": "123456"}'

# Now use wallet...
curl http://localhost:8080/scroll/wallet/balance

# Lock
curl -X POST http://localhost:8080/scroll/system/auth/lock
```

---

## Nostr Integration

### Get Nostr Identity

```rust
let node = NodeConfig::new("nostr-id")
    .with_mnemonic("your mnemonic here")
    .with_nostr(NostrConfig::default())
    .build()?;

// Get public key
let pubkey = node.get("/nostr/pubkey")?.unwrap();
println!("Pubkey: {}", pubkey.data["hex"]);

// Get human-readable mobi
let mobi = node.get("/nostr/mobi")?.unwrap();
println!("Mobi: {}", mobi.data["display"]);  // "123-456-789"
```

### Sign a Message

```rust
let signature = node.put("/nostr/sign", json!({
    "message": "Hello, Nostr!"
}))?;

println!("Signature: {}", signature.data["signature"]);
```

### Publish Event

```rust
// Connect to relays first
node.put("/nostr/connect", json!({}))?;

// Publish a note
node.put("/nostr/publish", json!({
    "kind": 1,
    "content": "Hello from Beenode!",
    "tags": []
}))?;
```

---

## Browser Integration

### Complete Web App Example

```html
<!DOCTYPE html>
<html>
<head>
    <title>Beenode Wallet</title>
</head>
<body>
    <div id="app">
        <h1>Beenode Wallet</h1>
        <div id="balance">Loading...</div>
        <button id="sync">Sync</button>
        <div id="address"></div>
    </div>

    <script type="module">
        import init, { BeeNode } from './pkg/beenode.js';

        let bee;

        async function main() {
            await init();
            bee = new BeeNode();
            await bee.init();

            // Display balance
            await updateBalance();

            // Display address
            const addr = await bee.read("/wallet/address");
            document.getElementById("address").textContent =
                `Receive: ${addr.data.address}`;

            // Sync button
            document.getElementById("sync").onclick = async () => {
                await bee.write("/wallet/sync", {});
                setTimeout(updateBalance, 3000);
            };

            // Watch for changes
            watchChanges();
        }

        async function updateBalance() {
            const balance = await bee.read("/wallet/balance");
            document.getElementById("balance").textContent =
                `Balance: ${balance.data.confirmed} sats`;
        }

        async function watchChanges() {
            for await (const scroll of bee.watch("/wallet/balance")) {
                document.getElementById("balance").textContent =
                    `Balance: ${scroll.data.confirmed} sats`;
            }
        }

        main();
    </script>
</body>
</html>
```

---

## Custom Namespace

### Implement a Simple Namespace

```rust
use beenode::{Namespace, Scroll, NineSResult};
use serde_json::{json, Value};

struct CounterNamespace {
    count: std::sync::atomic::AtomicU64,
}

impl Namespace for CounterNamespace {
    fn get(&self, path: &str) -> NineSResult<Option<Scroll>> {
        match path {
            "value" => Ok(Some(Scroll::new(
                "/counter/value",
                "counter/value@v1",
                json!({ "count": self.count.load(std::sync::atomic::Ordering::SeqCst) })
            ))),
            _ => Ok(None)
        }
    }

    fn put(&self, path: &str, data: Value) -> NineSResult<Scroll> {
        match path {
            "increment" => {
                let new_val = self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                Ok(Scroll::new("/counter/value", "counter/value@v1", json!({ "count": new_val })))
            }
            "reset" => {
                self.count.store(0, std::sync::atomic::Ordering::SeqCst);
                Ok(Scroll::new("/counter/value", "counter/value@v1", json!({ "count": 0 })))
            }
            _ => Err(beenode::NineSError::NotFound(path.to_string()))
        }
    }

    fn all(&self, _prefix: &str) -> NineSResult<Vec<String>> {
        Ok(vec!["/counter/value".to_string()])
    }
}

// Mount it
let shell = Shell::new()
    .mount("/counter", CounterNamespace { count: AtomicU64::new(0) });
```

---

## Testing with Scrolls

### Test a Workflow

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_workflow() {
        let node = TestNode::new();

        // Simulate initial state
        node.put("/wallet/balance", json!({
            "confirmed": 100000,
            "pending": 0,
            "spendable": 100000
        }));

        // Simulate send request
        node.put("/wallet/send", json!({
            "to": "tb1q...",
            "amount_sat": 20000
        }));

        // Verify effect was queued
        let paths = node.all("/external/bitcoin/send");
        assert_eq!(paths.len(), 1);

        // Simulate effect completion
        let effect_path = &paths[0];
        node.put(&format!("{}/result", effect_path), json!({
            "success": true,
            "txid": "abc123"
        }));

        // Verify balance updated
        node.put("/wallet/balance", json!({
            "confirmed": 79500,  // 100000 - 20000 - 500 fee
            "pending": 0,
            "spendable": 79500
        }));

        let balance = node.get("/wallet/balance").unwrap().unwrap();
        assert_eq!(balance.data["confirmed"], 79500);
    }
}
```

---

## Effect Handler

### Custom HTTP Webhook Effect

```rust
use beenode::{EffectHandler, Scroll, NineSResult};
use serde_json::Value;

struct WebhookEffectHandler {
    client: reqwest::Client,
}

impl EffectHandler for WebhookEffectHandler {
    fn watches(&self) -> &str {
        "/external/webhook/**"
    }

    async fn execute(&self, scroll: &Scroll) -> NineSResult<Value> {
        let url = scroll.data["url"].as_str()
            .ok_or_else(|| NineSError::InvalidData("missing url".into()))?;

        let payload = &scroll.data["payload"];

        let response = self.client
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|e| NineSError::Network(e.to_string()))?;

        Ok(json!({
            "status": response.status().as_u16(),
            "success": response.status().is_success()
        }))
    }
}
```
