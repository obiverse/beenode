//! WireGuard Key Derivation
//!
//! Derives Curve25519 keypairs from BIP39 mnemonic for WireGuard tunnels.
//! Enables secure tunnels between beenode clients and servers using
//! deterministic keys derived from the master mnemonic.
//!
//! ## Key Derivation Architecture
//!
//! ```text
//! Master Mnemonic (BIP39)
//!     │
//!     ├── BIP84 ────────────→ Bitcoin wallet
//!     ├── BIP85 ────────────→ Nostr keys
//!     ├── Mobi ─────────────→ Human-readable ID
//!     └── HMAC-SHA512 ──────→ WireGuard Curve25519 keypair
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use beenode::wireguard::{derive_keypair, public_key_to_base64};
//!
//! let keypair = derive_keypair("abandon abandon ...", None).unwrap();
//! println!("Public key: {}", public_key_to_base64(&keypair.public_key));
//! ```

mod namespace;

pub use namespace::WireGuardNamespace;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha512;
use thiserror::Error;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

/// WireGuard key errors
#[derive(Error, Debug)]
pub enum WireGuardError {
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    #[error("Key derivation failed: {0}")]
    DerivationFailed(String),

    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },

    #[error("Invalid base64: {0}")]
    InvalidBase64(String),
}

/// WireGuard keypair (Curve25519)
#[derive(Debug, Clone)]
pub struct WireGuardKeypair {
    /// Private key (32 bytes)
    pub private_key: [u8; 32],

    /// Public key (32 bytes)
    pub public_key: [u8; 32],
}

impl Zeroize for WireGuardKeypair {
    fn zeroize(&mut self) {
        self.private_key.zeroize();
    }
}

/// WireGuard tunnel configuration
#[derive(Debug, Clone, Default)]
pub struct WireGuardConfig {
    /// Client private key (32 bytes)
    pub private_key: [u8; 32],

    /// Server public key (32 bytes)
    pub server_public_key: [u8; 32],

    /// Server endpoint (host:port)
    pub server_endpoint: String,

    /// Assigned tunnel IP address (e.g., "10.21.0.42/32")
    pub tunnel_address: String,

    /// DNS servers (optional)
    pub dns: Option<Vec<String>>,

    /// Keepalive interval in seconds (default: 21)
    pub persistent_keepalive: u16,
}

impl WireGuardConfig {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Self {
            persistent_keepalive: 21, // Bitcoin's 21M
            ..Default::default()
        }
    }

    /// Set server endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.server_endpoint = endpoint.into();
        self
    }

    /// Set server public key from base64
    pub fn with_server_pubkey(mut self, b64: &str) -> Result<Self, WireGuardError> {
        self.server_public_key = base64_to_key(b64)?;
        Ok(self)
    }

    /// Set tunnel address
    pub fn with_address(mut self, addr: impl Into<String>) -> Self {
        self.tunnel_address = addr.into();
        self
    }

    /// Set DNS servers
    pub fn with_dns(mut self, dns: Vec<String>) -> Self {
        self.dns = Some(dns);
        self
    }

    /// Generate WireGuard config file format
    pub fn to_config_string(&self) -> String {
        let mut config = format!(
            "[Interface]\n\
             PrivateKey = {}\n\
             Address = {}\n",
            private_key_to_base64(&self.private_key),
            self.tunnel_address,
        );

        if let Some(dns) = &self.dns {
            config.push_str(&format!("DNS = {}\n", dns.join(", ")));
        }

        config.push_str(&format!(
            "\n[Peer]\n\
             PublicKey = {}\n\
             Endpoint = {}\n\
             AllowedIPs = 0.0.0.0/0\n\
             PersistentKeepalive = {}\n",
            public_key_to_base64(&self.server_public_key),
            self.server_endpoint,
            self.persistent_keepalive,
        ));

        config
    }
}

/// Domain separator for WireGuard key derivation
const WIREGUARD_DOMAIN: &[u8] = b"beenode-wireguard-v1";

/// Derive WireGuard entropy from mnemonic
///
/// Uses HMAC-SHA512 with a WireGuard-specific domain separator
/// to derive 32 bytes of entropy for the private key.
fn derive_wireguard_entropy(
    mnemonic: &str,
    passphrase: Option<&str>,
) -> Result<[u8; 32], WireGuardError> {
    use bip39::Mnemonic;

    // Parse mnemonic
    let mnemonic =
        Mnemonic::parse(mnemonic).map_err(|e| WireGuardError::InvalidMnemonic(e.to_string()))?;

    // Derive seed with optional passphrase
    let seed = mnemonic.to_seed(passphrase.unwrap_or(""));

    // Use HMAC-SHA512 with domain separator for WireGuard-specific entropy
    // This ensures WireGuard keys are isolated from Bitcoin/Nostr keys
    type HmacSha512 = Hmac<Sha512>;

    let mut hmac =
        HmacSha512::new_from_slice(WIREGUARD_DOMAIN).expect("HMAC accepts any key length");
    hmac.update(&seed);
    let result = hmac.finalize().into_bytes();

    let mut entropy = [0u8; 32];
    entropy.copy_from_slice(&result[..32]);

    Ok(entropy)
}

/// Derive WireGuard private key from mnemonic
///
/// Returns 32 bytes suitable for use as a Curve25519 private key.
pub fn derive_private_key(
    mnemonic: &str,
    passphrase: Option<&str>,
) -> Result<[u8; 32], WireGuardError> {
    derive_wireguard_entropy(mnemonic, passphrase)
}

/// Derive public key from private key using Curve25519
pub fn derive_public_key(private_key: &[u8; 32]) -> [u8; 32] {
    let secret = StaticSecret::from(*private_key);
    let public = PublicKey::from(&secret);
    *public.as_bytes()
}

/// Derive complete keypair from mnemonic
pub fn derive_keypair(
    mnemonic: &str,
    passphrase: Option<&str>,
) -> Result<WireGuardKeypair, WireGuardError> {
    let private_key = derive_private_key(mnemonic, passphrase)?;
    let public_key = derive_public_key(&private_key);

    Ok(WireGuardKeypair {
        private_key,
        public_key,
    })
}

/// Encode private key as base64 (for WireGuard config files)
pub fn private_key_to_base64(key: &[u8; 32]) -> String {
    BASE64.encode(key)
}

/// Encode public key as base64
pub fn public_key_to_base64(key: &[u8; 32]) -> String {
    BASE64.encode(key)
}

/// Decode key from base64
pub fn base64_to_key(b64: &str) -> Result<[u8; 32], WireGuardError> {
    let bytes = BASE64
        .decode(b64)
        .map_err(|e| WireGuardError::InvalidBase64(e.to_string()))?;

    if bytes.len() != 32 {
        return Err(WireGuardError::InvalidKeyLength {
            expected: 32,
            got: bytes.len(),
        });
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_derive_keypair() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();

        assert_eq!(keypair.private_key.len(), 32);
        assert_eq!(keypair.public_key.len(), 32);
        assert!(!keypair.public_key.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_deterministic_derivation() {
        let keypair1 = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let keypair2 = derive_keypair(TEST_MNEMONIC, None).unwrap();

        assert_eq!(keypair1.private_key, keypair2.private_key);
        assert_eq!(keypair1.public_key, keypair2.public_key);
    }

    #[test]
    fn test_passphrase_changes_keys() {
        let keypair1 = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let keypair2 = derive_keypair(TEST_MNEMONIC, Some("secret")).unwrap();

        assert_ne!(keypair1.private_key, keypair2.private_key);
        assert_ne!(keypair1.public_key, keypair2.public_key);
    }

    #[test]
    fn test_base64_roundtrip() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();

        let b64_priv = private_key_to_base64(&keypair.private_key);
        let b64_pub = public_key_to_base64(&keypair.public_key);

        let recovered_priv = base64_to_key(&b64_priv).unwrap();
        let recovered_pub = base64_to_key(&b64_pub).unwrap();

        assert_eq!(keypair.private_key, recovered_priv);
        assert_eq!(keypair.public_key, recovered_pub);
    }

    #[test]
    fn test_public_key_derivation() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let public_key = derive_public_key(&keypair.private_key);

        assert_eq!(keypair.public_key, public_key);
    }

    #[test]
    fn test_base64_format() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let b64 = public_key_to_base64(&keypair.public_key);

        // WireGuard base64 keys are 44 characters
        assert_eq!(b64.len(), 44);
        assert!(b64.ends_with('='));
    }

    #[test]
    fn test_config_generation() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();

        let config = WireGuardConfig {
            private_key: keypair.private_key,
            server_public_key: [0x42u8; 32],
            server_endpoint: "wg.beenode.net:51820".into(),
            tunnel_address: "10.21.0.42/32".into(),
            dns: Some(vec!["1.1.1.1".into()]),
            persistent_keepalive: 21,
        };

        let config_str = config.to_config_string();
        assert!(config_str.contains("[Interface]"));
        assert!(config_str.contains("[Peer]"));
        assert!(config_str.contains("PersistentKeepalive = 21"));
        assert!(config_str.contains("DNS = 1.1.1.1"));
    }

    #[test]
    fn test_keys_differ_from_seed() {
        // Ensure WireGuard keys are different from what you'd get
        // by just using the first 32 bytes of the seed directly
        let mnemonic =
            bip39::Mnemonic::parse(TEST_MNEMONIC).unwrap();
        let seed = mnemonic.to_seed("");

        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();

        // Private key should NOT be the first 32 bytes of seed
        assert_ne!(&keypair.private_key[..], &seed[..32]);
    }
}
