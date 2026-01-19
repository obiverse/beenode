# Security

Beenode's security model protects user funds and data through layered cryptographic protections.

## Threat Model

### What We Protect Against

1. **Unauthorized access** - Node requires PIN to unlock
2. **Data at rest exposure** - Mnemonic encrypted with AES-256-GCM
3. **Key extraction** - Seed never leaves keychain
4. **Network eavesdropping** - TLS for all network connections
5. **Replay attacks** - Nonces prevent ciphertext reuse

### What We Don't Protect Against

1. **Compromised device** - If attacker has root access, all bets are off
2. **Physical access with unlocked node** - Lock your node when not in use
3. **Weak PIN** - Use strong PINs; we can't enforce policy
4. **Social engineering** - User education required

## Cryptographic Primitives

| Purpose | Algorithm | Parameters |
|---------|-----------|------------|
| Key derivation | Argon2id | 64 MiB, 3 iterations, 4 threads |
| Encryption | AES-256-GCM | 256-bit key, 96-bit nonce |
| Hashing | BLAKE3 | 256-bit output |
| Mnemonic | BIP39 | 12/24 words, PBKDF2 for seed |
| Bitcoin keys | BIP84 | Native SegWit (m/84'/0'/0'/...) |
| Nostr keys | BIP85 | Derived child mnemonic |

## Key Hierarchy

```
BIP39 Mnemonic (12/24 words)
    │
    ▼
Master Seed (64 bytes)
    │
    ├──[BIP84]──► Bitcoin Keys
    │              └── m/84'/0'/0'/0/* (receive)
    │              └── m/84'/0'/0'/1/* (change)
    │
    └──[BIP85]──► Nostr Mnemonic (12 words)
                    └── NIP-06 derivation
                    └── secp256k1 keypair
```

### Key Isolation

- **Master seed** never leaves the keychain
- **Bitcoin signing keys** derived on-demand, not stored
- **Nostr keys** derived once per session
- **No key export** except explicit mnemonic backup

## Authentication

### PIN-Based Auth

1. User provides PIN (minimum 4 digits recommended)
2. Argon2id derives 256-bit key from PIN + random salt
3. BLAKE3 hashes derived key → verifier (stored)
4. AES-256-GCM encrypts mnemonic with derived key
5. Salt + verifier + ciphertext stored in auth file

```rust
// Simplified flow
let salt = generate_argon2_salt();  // 16 bytes random
let key = argon2id(pin, salt);      // 32 bytes
let verifier = blake3(key);         // For verification
let (nonce, ciphertext) = aes_gcm_encrypt(key, mnemonic, AAD);
```

### Unlock Flow

```
User enters PIN
    ↓
Argon2id(PIN, stored_salt) → derived_key
    ↓
BLAKE3(derived_key) → computed_verifier
    ↓
Compare computed_verifier == stored_verifier
    ↓
If match: AES-GCM decrypt mnemonic
    ↓
Derive keys, initialize wallet
```

### Locked State Behavior

When locked:
- All namespace operations blocked (except `/system/auth/*`)
- HTTP returns 401 Unauthorized
- Mnemonic remains encrypted in memory
- No keys available

## Encryption Details

### Mnemonic Encryption

- **AAD (Additional Authenticated Data)**: `b"beenode-mnemonic"`
- **Nonce**: 12 bytes, random per encryption
- **Format**: `nonce || ciphertext || tag`

### Storage Encryption

Native storage uses encrypted JSON files:

```
~/.beenode/{app}/
    ├── auth.json       # Encrypted mnemonic + verifier
    ├── store.db        # Encrypted scroll store
    └── wallet.sqlite   # BDK wallet database
```

## Network Security

### Electrum

- **Default**: SSL/TLS (`ssl://...`)
- Validates server certificate
- Connection over Tor supported (configure Electrum URL)

### Bitcoin RPC

- Only for regtest/local development
- Credentials via environment variables (not hardcoded)
- Never expose RPC to public network

### Nostr Relays

- WebSocket Secure (`wss://...`)
- Events signed with Nostr key
- No private data sent to relays (only signed events)

## Best Practices

### For Users

1. **Use strong PIN** - Minimum 6 digits, avoid patterns
2. **Backup mnemonic** - Write on paper, store securely
3. **Lock when idle** - Don't leave node unlocked
4. **Verify addresses** - Always verify receive addresses
5. **Test with testnet** - Use testnet before mainnet

### For Developers

1. **Never log sensitive data** - No mnemonics, keys, PINs in logs
2. **Use environment variables** - Not config files for secrets
3. **Validate all input** - Addresses, amounts, paths
4. **Handle errors** - Don't expose internal errors to users
5. **Keep dependencies updated** - `cargo audit` regularly

## Security Checklist

Before production deployment:

- [ ] PIN authentication enabled (`AuthMode::Pin`)
- [ ] Strong PIN set (6+ digits)
- [ ] Mnemonic backed up securely
- [ ] HTTPS/TLS for all network connections
- [ ] File permissions restricted (`chmod 600`)
- [ ] Environment variables for secrets
- [ ] `cargo audit` passes
- [ ] Node behind firewall (not public)

## Incident Response

### If Mnemonic Exposed

1. **Immediately** create new wallet
2. Transfer all funds to new wallet
3. Rotate all derived keys (Nostr, etc.)
4. Investigate how exposure occurred

### If PIN Compromised

1. Lock the node immediately
2. Change PIN (requires current PIN or mnemonic)
3. Consider creating new wallet if device compromised

### If Device Lost/Stolen

1. Funds are safe if:
   - Node was locked
   - Strong PIN was used
2. Create new wallet from mnemonic backup
3. Transfer funds to new wallet

## Vulnerability Disclosure

Report security vulnerabilities to: **security@obiverse.com**

Do NOT:
- Open public GitHub issues for security bugs
- Share vulnerability details publicly before fix

We will:
- Acknowledge within 48 hours
- Provide timeline for fix
- Credit reporter (if desired)

## Cryptographic Library Dependencies

| Library | Version | Purpose |
|---------|---------|---------|
| `argon2` | 0.5 | Password hashing |
| `aes-gcm` | 0.10 | Authenticated encryption |
| `blake3` | 1.5 | Fast hashing |
| `sha2` | 0.10 | SHA-256/512 |
| `hmac` | 0.12 | Message authentication |
| `bip39` | 2.0 | Mnemonic handling |
| `bitcoin` | 0.32 | Bitcoin primitives |
| `secp256k1` | (via bitcoin) | Elliptic curve |
| `rustls` | 0.23 | TLS (memory-safe) |

All libraries are well-maintained, widely audited Rust implementations.

## Audit Status

- [ ] Formal security audit (planned)
- [x] Code review by maintainers
- [x] Dependency audit (`cargo audit`)
- [x] Cryptographic review of primitives

## Appendix: Argon2id Parameters

```
Memory:      65536 KiB (64 MiB)
Iterations:  3
Parallelism: 4
Salt:        16 bytes (random)
Output:      32 bytes (256 bits)
```

These parameters:
- Exceed OWASP minimum recommendations (19 MiB)
- Resist GPU attacks (memory-hard)
- Complete in ~200ms on modern hardware
- Balance security vs. UX
