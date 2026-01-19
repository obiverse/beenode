# Quickstart

Get Beenode running in 5 minutes.

## Prerequisites

- Rust 1.75+ (`rustup update stable`)
- For wallet: Electrum server access (public servers available)
- For regtest: [Polar](https://lightningpolar.com/) (optional)

## Installation

### From Source

```bash
git clone https://github.com/obiverse/beenode.git
cd beenode
cargo build --release
```

### Features

```bash
# Full native build (default)
cargo build --release

# Minimal (no wallet/nostr)
cargo build --release --no-default-features

# WASM for browser
cargo build --release --target wasm32-unknown-unknown --features wasm
```

## Configuration

### Environment Variables

Create `.env` from the template:

```bash
cp .env.example .env
```

Required variables:

```bash
BEENODE_APP=myapp                    # Application name
BEENODE_NETWORK=testnet              # bitcoin, testnet, signet, regtest
BEENODE_MNEMONIC="your twelve word phrase here"
```

Optional:

```bash
BEENODE_PORT=8080                    # HTTP server port
ELECTRUM_URL=ssl://electrum.blockstream.info:60002
BITCOIN_RPC_URL=http://127.0.0.1:18443  # For regtest
BITCOIN_RPC_USER=user
BITCOIN_RPC_PASS=pass
```

### Generate a Mnemonic

```bash
# Generate new 12-word mnemonic
cargo run -- generate-mnemonic

# Or use bip39 tool
bip39 generate 12
```

## Running

### Server Mode

```bash
# Load .env and start server
cargo run --release -- serve

# Or with explicit config
BEENODE_APP=myapp \
BEENODE_NETWORK=testnet \
BEENODE_MNEMONIC="..." \
cargo run --release -- serve
```

Server starts at `http://localhost:8080`.

### Docker

```bash
# Copy docker env template
cp .env.docker.example .env.docker

# Edit with your mnemonic
nano .env.docker

# Run
docker compose up
```

### REPL Mode

```bash
cargo run -- repl
```

Interactive commands:

```
> get /wallet/balance
> put /wallet/sync {}
> list /wallet
> help
```

## First Steps

### 1. Check Health

```bash
curl http://localhost:8080/health
# {"status":"ok","service":"beenode"}
```

### 2. View Wallet Status

```bash
curl http://localhost:8080/scroll/wallet/status
# {"key":"/wallet/status","type":"wallet/status@v1","data":{"initialized":true,"network":"testnet"}}
```

### 3. Get Receive Address

```bash
curl http://localhost:8080/scroll/wallet/address
# {"data":{"address":"tb1q..."}}
```

### 4. Sync Wallet

```bash
curl -X POST http://localhost:8080/scroll/wallet/sync -d '{}'
# Triggers Electrum sync
```

### 5. Check Balance

```bash
curl http://localhost:8080/scroll/wallet/balance
# {"data":{"confirmed":0,"pending":0,"total":0,"spendable":0}}
```

## Testnet Faucet

Get testnet coins:

1. Get your address: `curl http://localhost:8080/scroll/wallet/address`
2. Visit [coinfaucet.eu](https://coinfaucet.eu/en/btc-testnet/) or [bitcoinfaucet.uo1.net](https://bitcoinfaucet.uo1.net/)
3. Paste your address, request coins
4. Sync: `curl -X POST http://localhost:8080/scroll/wallet/sync -d '{}'`
5. Check: `curl http://localhost:8080/scroll/wallet/balance`

## Regtest with Polar

For local development without real coins:

1. Install [Polar](https://lightningpolar.com/)
2. Create a network with Bitcoin Core
3. Start the network
4. Configure beenode:

```bash
BEENODE_NETWORK=regtest
BITCOIN_RPC_URL=http://127.0.0.1:18443
BITCOIN_RPC_USER=polaruser
BITCOIN_RPC_PASS=polarpass
```

5. Mine blocks in Polar to your beenode address

## Browser (WASM)

```html
<script type="module">
import init, { BeeNode } from './pkg/beenode.js';

await init();
const bee = new BeeNode();
await bee.init();

// Read
const balance = await bee.read("/wallet/balance");
console.log(balance);

// Write
await bee.write("/wallet/sync", {});

// Watch
for await (const scroll of bee.watch("/wallet/**")) {
    console.log("Changed:", scroll);
}
</script>
```

## Next Steps

- [ARCHITECTURE](ARCHITECTURE.md) - Understand the system design
- [API](API.md) - Full API reference
- [EXAMPLES](EXAMPLES.md) - Code patterns and recipes
- [SECURITY](SECURITY.md) - Security model and best practices
