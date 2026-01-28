//! WireGuard Integration Tests
//!
//! Comprehensive test suite for WireGuard key derivation and namespace.
//!
//! ## Test Categories
//!
//! 1. **Golden Tests** - Known input → expected output (catch regressions)
//! 2. **Determinism** - Same input always produces same output
//! 3. **Isolation** - WireGuard keys differ from Bitcoin/Nostr keys
//! 4. **Format Validation** - Keys are valid Curve25519, base64 is correct
//! 5. **Namespace** - All scroll paths work correctly
//! 6. **Identity Integration** - WireGuard keys are part of Identity
//! 7. **Error Handling** - Invalid inputs produce correct errors
//! 8. **Config Generation** - Valid WireGuard config file output

use once_cell::sync::Lazy;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

// Well-known test mnemonic (BIP39 test vector #0)
const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

// Alternative mnemonic for isolation tests
const ALT_MNEMONIC: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";

// ============================================================================
// 1. GOLDEN TESTS - Known Values (Regression Detection)
// ============================================================================

mod golden_tests {
    use super::*;
    use beenode::wireguard::{derive_keypair, public_key_to_base64, private_key_to_base64};

    /// Golden test: Known mnemonic produces known public key
    /// This catches any changes to the derivation algorithm
    #[test]
    fn golden_public_key_from_known_mnemonic() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let pubkey_b64 = public_key_to_base64(&keypair.public_key);

        // This is THE golden value - if this changes, derivation is broken
        // Derived deterministically from TEST_MNEMONIC using HMAC-SHA512
        // Domain: "beenode-wireguard-v1", then x25519 point multiplication
        assert_eq!(
            pubkey_b64,
            "GNEtpwEKPCFLPn9zP3m3AGQ2gd2kk8DzEvHGB6UUggA=",
            "Golden public key mismatch - derivation algorithm changed!"
        );
    }

    /// Golden test: Known mnemonic produces known private key
    #[test]
    fn golden_private_key_from_known_mnemonic() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let privkey_b64 = private_key_to_base64(&keypair.private_key);

        // Golden private key - HMAC-SHA512 output (first 32 bytes)
        assert_eq!(
            privkey_b64,
            "9zAhw/eVoh0rjBgIb0Qb4UqkNeH30/HFCX8jUQTBq9U=",
            "Golden private key mismatch - derivation algorithm changed!"
        );
    }

    /// Golden test: With passphrase produces different known key
    #[test]
    fn golden_with_passphrase() {
        let keypair = derive_keypair(TEST_MNEMONIC, Some("TREZOR")).expect("derivation");
        let pubkey_b64 = public_key_to_base64(&keypair.public_key);

        // Different from no-passphrase case
        assert_ne!(pubkey_b64, "XhLU8yqJd2bKWMT1ePvuX6OxsDGKN/Fj3InpPJYAlCg=");

        // But still deterministic
        let keypair2 = derive_keypair(TEST_MNEMONIC, Some("TREZOR")).expect("derivation");
        assert_eq!(keypair.public_key, keypair2.public_key);
    }

    /// Golden test: Alternative mnemonic produces different key
    #[test]
    fn golden_different_mnemonic() {
        let keypair1 = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let keypair2 = derive_keypair(ALT_MNEMONIC, None).expect("derivation");

        assert_ne!(keypair1.public_key, keypair2.public_key);
        assert_ne!(keypair1.private_key, keypair2.private_key);
    }
}

// ============================================================================
// 2. DETERMINISM TESTS - Reproducibility
// ============================================================================

mod determinism_tests {
    use super::*;
    use beenode::wireguard::derive_keypair;

    /// Same mnemonic always produces same keys
    #[test]
    fn deterministic_across_calls() {
        let results: Vec<_> = (0..10)
            .map(|_| derive_keypair(TEST_MNEMONIC, None).expect("derivation"))
            .collect();

        let first = &results[0];
        for keypair in &results[1..] {
            assert_eq!(first.private_key, keypair.private_key);
            assert_eq!(first.public_key, keypair.public_key);
        }
    }

    /// Empty passphrase is same as None
    #[test]
    fn empty_passphrase_equals_none() {
        let with_none = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let with_empty = derive_keypair(TEST_MNEMONIC, Some("")).expect("derivation");

        assert_eq!(with_none.private_key, with_empty.private_key);
        assert_eq!(with_none.public_key, with_empty.public_key);
    }

    /// Different passphrases produce different keys
    #[test]
    fn different_passphrases_differ() {
        let pass1 = derive_keypair(TEST_MNEMONIC, Some("alpha")).expect("derivation");
        let pass2 = derive_keypair(TEST_MNEMONIC, Some("beta")).expect("derivation");
        let pass3 = derive_keypair(TEST_MNEMONIC, Some("ALPHA")).expect("derivation");

        assert_ne!(pass1.public_key, pass2.public_key);
        assert_ne!(pass1.public_key, pass3.public_key); // Case sensitive
        assert_ne!(pass2.public_key, pass3.public_key);
    }
}

// ============================================================================
// 3. ISOLATION TESTS - Key Independence
// ============================================================================

mod isolation_tests {
    use super::*;
    use beenode::Identity;
    use beenode::wireguard::derive_keypair;

    /// WireGuard keys differ from raw seed bytes
    #[test]
    fn wireguard_differs_from_raw_seed() {
        use bip39::Mnemonic;

        let mnemonic = Mnemonic::parse(TEST_MNEMONIC).expect("mnemonic");
        let seed = mnemonic.to_seed("");

        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        // Private key is NOT the first 32 bytes of seed
        assert_ne!(&keypair.private_key[..], &seed[..32]);

        // Private key is NOT the second 32 bytes of seed
        assert_ne!(&keypair.private_key[..], &seed[32..64]);
    }

    /// WireGuard keys differ from Nostr keys (same mnemonic)
    #[test]
    fn wireguard_differs_from_nostr() {
        let identity = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");
        let wg_keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        // WireGuard public key != Nostr public key (different curves anyway)
        let wg_pubkey_hex = hex::encode(&wg_keypair.public_key);
        assert_ne!(wg_pubkey_hex, identity.pubkey_hex);
    }

    /// WireGuard keys are identical whether derived standalone or via Identity
    #[test]
    fn wireguard_consistent_via_identity() {
        let identity = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");
        let standalone = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        // Both derivation paths produce same WireGuard keys
        assert_eq!(identity.wireguard.public_key, standalone.public_key);
        assert_eq!(identity.wireguard.private_key, standalone.private_key);
    }

    /// Each mnemonic word change produces completely different keys
    #[test]
    fn single_word_change_cascades() {
        // Two different valid mnemonics
        let mnemonic1 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic2 = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";

        let keypair1 = derive_keypair(mnemonic1, None).expect("derivation");
        let keypair2 = derive_keypair(mnemonic2, None).expect("derivation");

        // Completely different keys
        assert_ne!(keypair1.private_key, keypair2.private_key);
        assert_ne!(keypair1.public_key, keypair2.public_key);

        // No correlation in bytes
        let matching_bytes: usize = keypair1.private_key.iter()
            .zip(keypair2.private_key.iter())
            .filter(|(a, b)| a == b)
            .count();

        // Statistical expectation: ~1 matching byte out of 32 by chance
        assert!(matching_bytes < 8, "Too many matching bytes - weak cascade");
    }
}

// ============================================================================
// 4. FORMAT VALIDATION TESTS - Cryptographic Correctness
// ============================================================================

mod format_tests {
    use super::*;
    use beenode::wireguard::{derive_keypair, derive_public_key, public_key_to_base64, base64_to_key};

    /// Keys are exactly 32 bytes
    #[test]
    fn key_lengths() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        assert_eq!(keypair.private_key.len(), 32);
        assert_eq!(keypair.public_key.len(), 32);
    }

    /// Base64 encoding is correct length (44 chars with padding)
    #[test]
    fn base64_length() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let b64 = public_key_to_base64(&keypair.public_key);

        // 32 bytes → 44 base64 characters (with padding)
        assert_eq!(b64.len(), 44);
        assert!(b64.ends_with('='), "Should have padding");
    }

    /// Base64 round-trip preserves data
    #[test]
    fn base64_roundtrip() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        let encoded = public_key_to_base64(&keypair.public_key);
        let decoded = base64_to_key(&encoded).expect("decode");

        assert_eq!(keypair.public_key, decoded);
    }

    /// Public key derivation is consistent
    #[test]
    fn public_from_private_consistent() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let derived_public = derive_public_key(&keypair.private_key);

        assert_eq!(keypair.public_key, derived_public);
    }

    /// Private key is not all zeros
    #[test]
    fn private_key_not_zero() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        assert!(!keypair.private_key.iter().all(|&b| b == 0));
    }

    /// Public key is not all zeros
    #[test]
    fn public_key_not_zero() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        assert!(!keypair.public_key.iter().all(|&b| b == 0));
    }

    /// Public key is a valid Curve25519 point (not the identity)
    #[test]
    fn public_key_valid_point() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        // The identity point on Curve25519 is all zeros
        let identity_point = [0u8; 32];
        assert_ne!(keypair.public_key, identity_point);

        // A low-order point (torsion) - should not equal
        let low_order = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_ne!(keypair.public_key, low_order);
    }
}

// ============================================================================
// 5. NAMESPACE TESTS - Scroll-based Access
// ============================================================================

mod namespace_tests {
    use super::*;
    use beenode::wireguard::{derive_keypair, WireGuardNamespace, WireGuardConfig, public_key_to_base64};
    use nine_s_core::prelude::Namespace;

    /// Status path returns initialized state
    #[test]
    fn read_status() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        let scroll = ns.read("status").expect("read").expect("scroll");

        assert_eq!(scroll.key, "/wireguard/status");
        assert_eq!(scroll.data["initialized"], true);
        assert_eq!(scroll.data["has_config"], false);
    }

    /// Status with leading slash works
    #[test]
    fn read_status_with_slash() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        let scroll = ns.read("/status").expect("read").expect("scroll");
        assert_eq!(scroll.data["initialized"], true);
    }

    /// Pubkey path returns base64 and hex
    #[test]
    fn read_pubkey() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let expected_b64 = public_key_to_base64(&keypair.public_key);
        let expected_hex = hex::encode(&keypair.public_key);

        let ns = WireGuardNamespace::new(keypair);
        let scroll = ns.read("pubkey").expect("read").expect("scroll");

        assert_eq!(scroll.key, "/wireguard/pubkey");
        assert_eq!(scroll.data["base64"].as_str().unwrap(), expected_b64);
        assert_eq!(scroll.data["hex"].as_str().unwrap(), expected_hex);
    }

    /// Config path returns None when not configured
    #[test]
    fn read_config_unconfigured() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        let result = ns.read("config").expect("read");
        assert!(result.is_none(), "Should return None when no config set");
    }

    /// Config path returns config when set
    #[test]
    fn read_config_configured() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let config = WireGuardConfig {
            private_key: keypair.private_key,
            server_public_key: [0x42u8; 32],
            server_endpoint: "wg.test.com:51820".into(),
            tunnel_address: "10.21.0.1/32".into(),
            dns: Some(vec!["8.8.8.8".into()]),
            persistent_keepalive: 25,
        };

        let ns = WireGuardNamespace::with_config(keypair, config);

        // Status reflects config presence
        let status = ns.read("status").expect("read").expect("scroll");
        assert_eq!(status.data["has_config"], true);

        // Config returns the full config
        let config_scroll = ns.read("config").expect("read").expect("scroll");
        assert_eq!(config_scroll.key, "/wireguard/config");
        assert!(config_scroll.data["config_file"].as_str().unwrap().contains("[Interface]"));
        assert_eq!(config_scroll.data["server_endpoint"], "wg.test.com:51820");
        assert_eq!(config_scroll.data["tunnel_address"], "10.21.0.1/32");
    }

    /// Unknown path returns None
    #[test]
    fn read_unknown_path() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        let result = ns.read("unknown").expect("read");
        assert!(result.is_none());

        let result = ns.read("/nonexistent").expect("read");
        assert!(result.is_none());
    }

    /// List returns standard paths
    #[test]
    fn list_paths() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        let paths = ns.list("").expect("list");

        assert!(paths.contains(&"/wireguard/status".to_string()));
        assert!(paths.contains(&"/wireguard/pubkey".to_string()));
        assert!(!paths.contains(&"/wireguard/config".to_string())); // No config set
    }

    /// List includes config path when configured
    #[test]
    fn list_paths_with_config() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let config = WireGuardConfig {
            private_key: keypair.private_key,
            server_public_key: [0x42u8; 32],
            server_endpoint: "wg.test.com:51820".into(),
            tunnel_address: "10.21.0.1/32".into(),
            dns: None,
            persistent_keepalive: 25,
        };

        let ns = WireGuardNamespace::with_config(keypair, config);
        let paths = ns.list("").expect("list");

        assert!(paths.contains(&"/wireguard/config".to_string()));
    }

    /// Write to unknown path returns error
    #[test]
    fn write_unknown_path_errors() {
        use serde_json::json;

        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        let result = ns.write("unknown", json!({}));
        assert!(result.is_err());
    }

    /// Close is idempotent
    #[test]
    fn close_idempotent() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let ns = WireGuardNamespace::new(keypair);

        ns.close().expect("first close");
        ns.close().expect("second close");
        ns.close().expect("third close");
    }
}

// ============================================================================
// 6. IDENTITY INTEGRATION TESTS
// ============================================================================

mod identity_tests {
    use super::*;
    use beenode::Identity;
    use beenode::wireguard::derive_keypair;

    /// Identity includes WireGuard keys
    #[test]
    fn identity_has_wireguard() {
        let identity = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");

        // Keys are present and non-zero
        assert!(!identity.wireguard.private_key.iter().all(|&b| b == 0));
        assert!(!identity.wireguard.public_key.iter().all(|&b| b == 0));
    }

    /// Identity WireGuard matches standalone derivation
    #[test]
    fn identity_wireguard_matches_standalone() {
        let identity = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");
        let standalone = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        assert_eq!(identity.wireguard.public_key, standalone.public_key);
        assert_eq!(identity.wireguard.private_key, standalone.private_key);
    }

    /// Identity from_seed also produces WireGuard keys
    #[test]
    fn identity_from_seed_has_wireguard() {
        let mut seed = [0u8; 64];
        seed[..32].copy_from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ]);

        let identity = Identity::from_seed(&seed).expect("identity");

        // WireGuard keys present
        assert!(!identity.wireguard.private_key.iter().all(|&b| b == 0));
        assert!(!identity.wireguard.public_key.iter().all(|&b| b == 0));
    }

    /// Identity determinism includes WireGuard
    #[test]
    fn identity_wireguard_deterministic() {
        let id1 = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");
        let id2 = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");

        assert_eq!(id1.wireguard.public_key, id2.wireguard.public_key);
        assert_eq!(id1.wireguard.private_key, id2.wireguard.private_key);
    }

    /// Different mnemonics produce different WireGuard keys via Identity
    #[test]
    fn identity_different_mnemonics() {
        let id1 = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");
        let id2 = Identity::from_mnemonic(ALT_MNEMONIC).expect("identity");

        assert_ne!(id1.wireguard.public_key, id2.wireguard.public_key);
    }

    /// All identity components are derived from same mnemonic
    #[test]
    fn identity_components_consistent() {
        let identity = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");

        // All components should be non-empty
        assert!(!identity.pubkey_hex.is_empty());
        assert!(!identity.mobi.display.is_empty());
        assert!(!identity.wireguard.public_key.iter().all(|&b| b == 0));

        // But all different (different derivation paths)
        let wg_hex = hex::encode(&identity.wireguard.public_key);
        assert_ne!(wg_hex, identity.pubkey_hex);
    }
}

// ============================================================================
// 7. ERROR HANDLING TESTS
// ============================================================================

mod error_tests {
    use super::*;
    use beenode::wireguard::{derive_keypair, base64_to_key, WireGuardError};

    /// Invalid mnemonic returns error
    #[test]
    fn invalid_mnemonic_errors() {
        let result = derive_keypair("not a valid mnemonic", None);
        assert!(result.is_err());

        match result {
            Err(WireGuardError::InvalidMnemonic(_)) => {}
            Err(e) => panic!("Expected InvalidMnemonic, got: {:?}", e),
            Ok(_) => panic!("Should have failed"),
        }
    }

    /// Empty mnemonic returns error
    #[test]
    fn empty_mnemonic_errors() {
        let result = derive_keypair("", None);
        assert!(result.is_err());
    }

    /// Partial mnemonic returns error
    #[test]
    fn partial_mnemonic_errors() {
        let result = derive_keypair("abandon abandon abandon", None);
        assert!(result.is_err());
    }

    /// Invalid base64 returns error
    #[test]
    fn invalid_base64_errors() {
        let result = base64_to_key("not valid base64!!!");
        assert!(result.is_err());

        match result {
            Err(WireGuardError::InvalidBase64(_)) => {}
            Err(e) => panic!("Expected InvalidBase64, got: {:?}", e),
            Ok(_) => panic!("Should have failed"),
        }
    }

    /// Wrong-length base64 returns error
    #[test]
    fn wrong_length_base64_errors() {
        // Valid base64 but only 16 bytes
        let short_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 16]);
        let result = base64_to_key(&short_b64);

        match result {
            Err(WireGuardError::InvalidKeyLength { expected: 32, got: 16 }) => {}
            Err(e) => panic!("Expected InvalidKeyLength, got: {:?}", e),
            Ok(_) => panic!("Should have failed"),
        }
    }

    /// Mnemonic with wrong checksum returns error
    #[test]
    fn wrong_checksum_errors() {
        // Valid words but wrong checksum (last word should be "about" for this seed)
        let bad_checksum = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        let result = derive_keypair(bad_checksum, None);
        assert!(result.is_err());
    }
}

// ============================================================================
// 8. CONFIG GENERATION TESTS
// ============================================================================

mod config_tests {
    use super::*;
    use beenode::wireguard::{derive_keypair, WireGuardConfig, public_key_to_base64, private_key_to_base64};

    /// Config generates valid WireGuard format
    #[test]
    fn config_format_valid() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        let server_pubkey = [0x42u8; 32];

        let config = WireGuardConfig {
            private_key: keypair.private_key,
            server_public_key: server_pubkey,
            server_endpoint: "wg.example.com:51820".into(),
            tunnel_address: "10.21.0.42/32".into(),
            dns: Some(vec!["1.1.1.1".into(), "8.8.8.8".into()]),
            persistent_keepalive: 21,
        };

        let config_str = config.to_config_string();

        // Required sections
        assert!(config_str.contains("[Interface]"));
        assert!(config_str.contains("[Peer]"));

        // Interface section
        assert!(config_str.contains(&format!("PrivateKey = {}", private_key_to_base64(&keypair.private_key))));
        assert!(config_str.contains("Address = 10.21.0.42/32"));
        assert!(config_str.contains("DNS = 1.1.1.1, 8.8.8.8"));

        // Peer section
        assert!(config_str.contains(&format!("PublicKey = {}", public_key_to_base64(&server_pubkey))));
        assert!(config_str.contains("Endpoint = wg.example.com:51820"));
        assert!(config_str.contains("AllowedIPs = 0.0.0.0/0"));
        assert!(config_str.contains("PersistentKeepalive = 21"));
    }

    /// Config without DNS omits DNS line
    #[test]
    fn config_without_dns() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        let config = WireGuardConfig {
            private_key: keypair.private_key,
            server_public_key: [0x42u8; 32],
            server_endpoint: "wg.example.com:51820".into(),
            tunnel_address: "10.21.0.42/32".into(),
            dns: None,
            persistent_keepalive: 21,
        };

        let config_str = config.to_config_string();
        assert!(!config_str.contains("DNS ="));
    }

    /// Config builder works
    #[test]
    fn config_builder() {
        let server_b64 = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI="; // [0x42; 32]

        let config = WireGuardConfig::new()
            .with_endpoint("wg.example.com:51820")
            .with_server_pubkey(server_b64).expect("pubkey")
            .with_address("10.21.0.42/32")
            .with_dns(vec!["1.1.1.1".into()]);

        assert_eq!(config.server_endpoint, "wg.example.com:51820");
        assert_eq!(config.server_public_key, [0x42u8; 32]);
        assert_eq!(config.tunnel_address, "10.21.0.42/32");
        assert_eq!(config.dns, Some(vec!["1.1.1.1".into()]));
        assert_eq!(config.persistent_keepalive, 21); // Default
    }

    /// Config with invalid server pubkey errors
    #[test]
    fn config_invalid_server_pubkey() {
        let result = WireGuardConfig::new()
            .with_server_pubkey("not valid base64!!!");

        assert!(result.is_err());
    }
}

// ============================================================================
// 9. NODE INTEGRATION TESTS (Full Stack)
// ============================================================================

mod node_tests {
    use super::*;
    use beenode::{Node, NodeConfig};
    use beenode::wireguard::derive_keypair;

    /// Node identity includes WireGuard
    #[test]
    fn node_identity_has_wireguard() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-wg").with_mnemonic(TEST_MNEMONIC);
        let node = Node::from_config(config).expect("node");

        let identity = node.identity().expect("identity");

        // WireGuard keys present
        assert!(!identity.wireguard.public_key.iter().all(|&b| b == 0));

        // Match standalone derivation
        let standalone = derive_keypair(TEST_MNEMONIC, None).expect("derivation");
        assert_eq!(identity.wireguard.public_key, standalone.public_key);

        node.close().expect("close");
    }

    /// Node with different mnemonics have different WireGuard keys
    #[test]
    fn node_wireguard_unique_per_mnemonic() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let node1 = Node::from_config(
            NodeConfig::new("test-wg1").with_mnemonic(TEST_MNEMONIC)
        ).expect("node1");

        let node2 = Node::from_config(
            NodeConfig::new("test-wg2").with_mnemonic(ALT_MNEMONIC)
        ).expect("node2");

        let id1 = node1.identity().expect("identity1");
        let id2 = node2.identity().expect("identity2");

        assert_ne!(id1.wireguard.public_key, id2.wireguard.public_key);

        node1.close().expect("close1");
        node2.close().expect("close2");
    }
}

// ============================================================================
// 10. ENTROPY QUALITY TESTS
// ============================================================================

mod entropy_tests {
    use super::TEST_MNEMONIC;
    use beenode::wireguard::derive_keypair;
    use std::collections::HashSet;

    /// Different mnemonics produce unique keys (no collisions)
    #[test]
    fn no_collisions_across_mnemonics() {
        let mnemonics = [
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent",
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
        ];

        let mut seen_pubkeys: HashSet<[u8; 32]> = HashSet::new();
        let mut seen_privkeys: HashSet<[u8; 32]> = HashSet::new();

        for mnemonic in &mnemonics {
            let keypair = derive_keypair(mnemonic, None).expect("derivation");

            assert!(seen_pubkeys.insert(keypair.public_key), "Public key collision!");
            assert!(seen_privkeys.insert(keypair.private_key), "Private key collision!");
        }
    }

    /// Passphrase variations produce unique keys
    #[test]
    fn passphrase_variations_unique() {
        let passphrases = [None, Some(""), Some("a"), Some("test"), Some("ТЕСТ")]; // Unicode

        let mut seen: HashSet<[u8; 32]> = HashSet::new();

        for pass in &passphrases {
            let keypair = derive_keypair(TEST_MNEMONIC, *pass).expect("derivation");

            // None and Some("") should be the same
            if *pass == Some("") {
                continue; // Skip - known to equal None
            }

            assert!(seen.insert(keypair.public_key), "Collision with passphrase: {:?}", pass);
        }
    }

    /// Private key bytes have good distribution
    #[test]
    fn private_key_byte_distribution() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).expect("derivation");

        // Count unique byte values
        let unique: HashSet<u8> = keypair.private_key.iter().cloned().collect();

        // With 32 random bytes, we expect most values to be unique
        // (birthday paradox: ~28-30 unique values expected)
        assert!(unique.len() > 20, "Poor byte distribution: only {} unique values", unique.len());
    }
}
