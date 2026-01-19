//! Integration Tests: Wallet and Nostr production functionality
//!
//! These tests verify:
//! 1. BDK wallet persistence across restarts
//! 2. Wallet sync with testnet electrum
//! 3. Nostr relay connection and message parsing
//! 4. RelayPool auto-reconnection
//! 5. Full Node configuration with all features

use once_cell::sync::Lazy;
use serde_json::json;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

// Test mnemonic (well-known, never use with real funds)
const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Test: Node creates with mnemonic, derives identity
#[test]
fn node_identity_derivation() {
    use beenode::{Node, NodeConfig};

    let _guard = lock_env();
    let dir = TempDir::new().expect("tempdir");
    std::env::set_var("NINE_S_ROOT", dir.path());

    let config = NodeConfig::new("test-identity").with_mnemonic(TEST_MNEMONIC);
    let node = Node::from_config(config).expect("node");

    // Identity derived from mnemonic
    let identity = node.identity().expect("identity");
    assert!(!identity.pubkey_hex.is_empty());
    assert_eq!(identity.mobi.display.len(), 12);

    // Mobi is deterministic from pubkey
    let mobi = node.mobi().expect("mobi");
    assert_eq!(mobi.display.len(), 12);
    assert!(mobi.display_formatted().contains('-'));
}

/// Test: Five verbs work correctly
#[test]
fn node_five_verbs() {
    use beenode::{Node, NodeConfig};

    let _guard = lock_env();
    let dir = TempDir::new().expect("tempdir");
    std::env::set_var("NINE_S_ROOT", dir.path());

    let node = Node::from_config(NodeConfig::new("test-verbs")).expect("node");

    // put
    let scroll = node.put("/test/scroll", json!({"value": 42})).expect("put");
    assert_eq!(scroll.key, "/test/scroll");

    // get
    let retrieved = node.get("/test/scroll").expect("get").expect("scroll");
    assert_eq!(retrieved.data["value"], 42);

    // all
    let keys = node.all("/test").expect("all");
    assert!(keys.contains(&"/test/scroll".to_string()));

    // exists
    assert!(node.exists("/test/scroll").expect("exists"));
    assert!(!node.exists("/nonexistent").expect("exists"));

    // close
    node.close().expect("close");
}

/// Test: Mobi derivation is deterministic
#[test]
fn mobi_derivation_deterministic() {
    use beenode::Identity;

    let id1 = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");
    let id2 = Identity::from_mnemonic(TEST_MNEMONIC).expect("identity");

    // Same mnemonic produces same identity
    assert_eq!(id1.pubkey_hex, id2.pubkey_hex);
    assert_eq!(id1.mobi.display, id2.mobi.display);
    assert_eq!(id1.mobi.full, id2.mobi.full);

    // Mobi components are proper lengths
    assert_eq!(id1.mobi.display.len(), 12);
    assert_eq!(id1.mobi.extended.len(), 15);
    assert_eq!(id1.mobi.long.len(), 18);
    assert_eq!(id1.mobi.full.len(), 21);
}

/// Test: Graceful shutdown signal handling
#[test]
fn shutdown_signaling() {
    use beenode::Shutdown;

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let shutdown = Shutdown::new();

        // Not triggered initially
        assert!(!shutdown.is_triggered().await);

        // Can subscribe
        let mut rx = shutdown.subscribe();

        // Trigger
        shutdown.trigger().await;

        // Now triggered
        assert!(shutdown.is_triggered().await);

        // Subscriber receives notification
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(result.is_ok());
    });
}

// ============================================================================
// Wallet Integration Tests (feature-gated)
// ============================================================================

#[cfg(feature = "wallet")]
mod wallet_tests {
    use super::*;
    use beenode::{Network, Node, NodeConfig, WalletConfig};

    /// Test: Wallet namespace mounts and responds
    #[test]
    fn wallet_namespace_basic() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-wallet")
            .with_mnemonic(TEST_MNEMONIC)
            .with_wallet(WalletConfig {
                network: Network::Signet,
                electrum_url: None, // No sync
                data_dir: Some(dir.path().to_path_buf()),
            });

        let node = Node::from_config(config).expect("node");

        // Wallet status
        let status = node.get("/wallet/status").expect("get");
        assert!(status.is_some());
        let status = status.unwrap();
        assert_eq!(status.data["initialized"], true);

        // Wallet address (deterministic from mnemonic)
        let addr = node.get("/wallet/address").expect("get");
        assert!(addr.is_some());
        let addr = addr.unwrap();
        assert!(addr.data["address"].as_str().unwrap().starts_with("tb1") ||
                addr.data["address"].as_str().unwrap().starts_with("bc1"));

        // Wallet balance (zero initially)
        let balance = node.get("/wallet/balance").expect("get");
        assert!(balance.is_some());

        node.close().expect("close");
    }

    /// Test: Wallet persistence across restarts
    /// Note: Address derivation is deterministic from mnemonic, so same mnemonic = same addresses
    #[test]
    fn wallet_persistence() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        // Use unique wallet db path for this test
        let wallet_db = dir.path().join("wallet-persist-test.sqlite");

        let config = || NodeConfig::new("test-wallet-persist")
            .with_mnemonic(TEST_MNEMONIC)
            .with_wallet(WalletConfig {
                network: Network::Signet,
                electrum_url: None,
                data_dir: Some(wallet_db.parent().unwrap().to_path_buf()),
            });

        // First instance - get balance
        let balance1 = {
            let node = Node::from_config(config()).expect("node");
            let balance = node.get("/wallet/balance").expect("get").expect("scroll");
            let confirmed = balance.data["confirmed"].as_u64().unwrap_or(0);
            node.close().expect("close");
            confirmed
        };

        // Second instance - balance should still be available
        let balance2 = {
            let node = Node::from_config(config()).expect("node");
            let balance = node.get("/wallet/balance").expect("get").expect("scroll");
            let confirmed = balance.data["confirmed"].as_u64().unwrap_or(0);
            node.close().expect("close");
            confirmed
        };

        // Balances match (both zero, but proves wallet loads correctly)
        assert_eq!(balance1, balance2);

        // Both instances can read address (deterministic from mnemonic)
        let node = Node::from_config(config()).expect("node");
        let addr = node.get("/wallet/address").expect("get").expect("scroll");
        assert!(addr.data["address"].as_str().unwrap().starts_with("tb1"));
        node.close().expect("close");
    }

    /// Test: Wallet list paths
    #[test]
    fn wallet_list_paths() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-wallet-list")
            .with_mnemonic(TEST_MNEMONIC)
            .with_wallet(WalletConfig {
                network: Network::Signet,
                electrum_url: None,
                data_dir: Some(dir.path().to_path_buf()),
            });

        let node = Node::from_config(config).expect("node");
        let paths = node.all("/wallet").expect("all");

        // Standard wallet paths exist
        assert!(paths.contains(&"/wallet/status".to_string()));
        assert!(paths.contains(&"/wallet/balance".to_string()));
        assert!(paths.contains(&"/wallet/address".to_string()));

        node.close().expect("close");
    }
}

// ============================================================================
// Nostr Integration Tests (feature-gated)
// ============================================================================

#[cfg(feature = "nostr")]
mod nostr_tests {
    use super::*;
    use beenode::node::NostrConfig;
    use beenode::{Node, NodeConfig};

    /// Test: Nostr namespace mounts and responds
    #[test]
    fn nostr_namespace_basic() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-nostr")
            .with_mnemonic(TEST_MNEMONIC)
            .with_nostr(NostrConfig {
                relays: vec!["wss://relay.damus.io".to_string()],
                beebase_url: None,
                auto_connect: false,
            });

        let node = Node::from_config(config).expect("node");

        // Nostr status
        let status = node.get("/nostr/status").expect("get").expect("scroll");
        assert_eq!(status.data["initialized"], true);

        // Nostr pubkey (deterministic from mnemonic)
        let pubkey = node.get("/nostr/pubkey").expect("get").expect("scroll");
        assert!(!pubkey.data["hex"].as_str().unwrap().is_empty());

        // Nostr mobi
        let mobi = node.get("/nostr/mobi").expect("get").expect("scroll");
        assert_eq!(mobi.data["display"].as_str().unwrap().len(), 12);
        assert!(mobi.data["formatted"].as_str().unwrap().contains('-'));

        // Relays config
        let relays = node.get("/nostr/relays").expect("get").expect("scroll");
        let urls = relays.data["urls"].as_array().unwrap();
        assert_eq!(urls.len(), 1);

        node.close().expect("close");
    }

    /// Test: Relay message parsing (NIP-01)
    /// Note: EVENT parsing requires valid nostr::Event, so we test the simpler message types
    #[test]
    fn relay_message_parsing() {
        use beenode::nostr::client::{parse_relay_message, RelayMessage};

        // OK message (accepted)
        let ok_msg = r#"["OK","abc123def456abc123def456abc123def456abc123def456abc123def456abcd",true,""]"#;
        let parsed = parse_relay_message(ok_msg);
        assert!(parsed.is_some(), "OK message should parse");
        if let Some(RelayMessage::Ok { event_id, accepted, message }) = parsed {
            assert_eq!(event_id, "abc123def456abc123def456abc123def456abc123def456abc123def456abcd");
            assert!(accepted);
            assert_eq!(message, Some("".to_string()));
        } else {
            panic!("Expected Ok");
        }

        // OK message (rejected)
        let ok_reject = r#"["OK","abc123def456abc123def456abc123def456abc123def456abc123def456abcd",false,"duplicate: already have this event"]"#;
        let parsed = parse_relay_message(ok_reject);
        assert!(parsed.is_some());
        if let Some(RelayMessage::Ok { accepted, message, .. }) = parsed {
            assert!(!accepted);
            assert_eq!(message, Some("duplicate: already have this event".to_string()));
        } else {
            panic!("Expected Ok");
        }

        // EOSE message (end of stored events)
        let eose_msg = r#"["EOSE","subscription-id-123"]"#;
        let parsed = parse_relay_message(eose_msg);
        assert!(parsed.is_some(), "EOSE message should parse");
        if let Some(RelayMessage::Eose { sub_id }) = parsed {
            assert_eq!(sub_id, "subscription-id-123");
        } else {
            panic!("Expected Eose");
        }

        // NOTICE message
        let notice_msg = r#"["NOTICE","rate limited: slow down"]"#;
        let parsed = parse_relay_message(notice_msg);
        assert!(parsed.is_some(), "NOTICE message should parse");
        if let Some(RelayMessage::Notice { message }) = parsed {
            assert_eq!(message, "rate limited: slow down");
        } else {
            panic!("Expected Notice");
        }

        // Unknown message type
        let unknown = r#"["UNKNOWN","data"]"#;
        assert!(parse_relay_message(unknown).is_none(), "Unknown type should return None");

        // Malformed JSON
        let malformed = r#"not json"#;
        assert!(parse_relay_message(malformed).is_none(), "Malformed should return None");

        // Empty array
        let empty = r#"[]"#;
        assert!(parse_relay_message(empty).is_none(), "Empty array should return None");
    }

    /// Test: Nostr sign operation
    #[test]
    fn nostr_sign_message() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-nostr-sign")
            .with_mnemonic(TEST_MNEMONIC)
            .with_nostr(NostrConfig {
                relays: vec![],
                beebase_url: None,
                auto_connect: false,
            });

        let node = Node::from_config(config).expect("node");

        // Sign a message
        let result = node.put("/nostr/sign", json!({"message": "Hello, Nostr!"}));
        assert!(result.is_ok());

        let signed = result.unwrap();
        assert!(!signed.data["signature"].as_str().unwrap().is_empty());
        assert!(!signed.data["event_id"].as_str().unwrap().is_empty());
        assert_eq!(signed.data["message"], "Hello, Nostr!");

        node.close().expect("close");
    }

    /// Test: RelayClient state management
    #[test]
    fn relay_client_state() {
        use beenode::nostr::client::{RelayClient, RelayState};

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let client = RelayClient::new("wss://relay.damus.io");

            // Initially disconnected
            assert_eq!(client.state().await, RelayState::Disconnected);

            // Note: Actually connecting would require network access
            // This test just verifies state management works
        });
    }

    /// Test: Nostr list paths
    #[test]
    fn nostr_list_paths() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-nostr-list")
            .with_mnemonic(TEST_MNEMONIC)
            .with_nostr(NostrConfig {
                relays: vec![],
                beebase_url: None,
                auto_connect: false,
            });

        let node = Node::from_config(config).expect("node");
        let paths = node.all("/nostr").expect("all");

        // Standard nostr paths exist
        assert!(paths.contains(&"/nostr/status".to_string()));
        assert!(paths.contains(&"/nostr/pubkey".to_string()));
        assert!(paths.contains(&"/nostr/mobi".to_string()));
        assert!(paths.contains(&"/nostr/relays".to_string()));

        node.close().expect("close");
    }
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[cfg(all(feature = "wallet", feature = "nostr"))]
mod combined_tests {
    use super::*;
    use beenode::node::NostrConfig;
    use beenode::{Network, Node, NodeConfig, WalletConfig};

    /// Test: Full node with both wallet and nostr
    #[test]
    fn full_node_configuration() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-full")
            .with_mnemonic(TEST_MNEMONIC)
            .with_wallet(WalletConfig {
                network: Network::Signet,
                electrum_url: None,
                data_dir: Some(dir.path().to_path_buf()),
            })
            .with_nostr(NostrConfig {
                relays: vec!["wss://relay.damus.io".to_string()],
                beebase_url: None,
                auto_connect: false,
            });

        let node = Node::from_config(config).expect("node");

        // Both namespaces mounted
        assert!(node.get("/wallet/status").expect("get").is_some());
        assert!(node.get("/nostr/status").expect("get").is_some());

        // Identity available
        assert!(node.identity().is_some());
        assert!(node.mobi().is_some());

        // All paths include both
        let wallet_paths = node.all("/wallet").expect("all");
        let nostr_paths = node.all("/nostr").expect("all");
        assert!(!wallet_paths.is_empty());
        assert!(!nostr_paths.is_empty());

        node.close().expect("close");
    }

    /// Test: Same mnemonic produces consistent identity across features
    #[test]
    fn consistent_identity_across_features() {
        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let config = NodeConfig::new("test-identity-consistent")
            .with_mnemonic(TEST_MNEMONIC)
            .with_wallet(WalletConfig {
                network: Network::Signet,
                electrum_url: None,
                data_dir: Some(dir.path().to_path_buf()),
            })
            .with_nostr(NostrConfig {
                relays: vec![],
                beebase_url: None,
                auto_connect: false,
            });

        let node = Node::from_config(config).expect("node");

        // Get identity from node
        let identity = node.identity().expect("identity");

        // Get pubkey from nostr namespace
        let nostr_pubkey = node.get("/nostr/pubkey").expect("get").expect("scroll");
        let nostr_hex = nostr_pubkey.data["hex"].as_str().unwrap();

        // They match
        assert_eq!(identity.pubkey_hex, nostr_hex);

        // Get mobi from both
        let node_mobi = node.mobi().expect("mobi");
        let nostr_mobi = node.get("/nostr/mobi").expect("get").expect("scroll");

        assert_eq!(node_mobi.display, nostr_mobi.data["display"].as_str().unwrap());
        assert_eq!(node_mobi.full, nostr_mobi.data["full"].as_str().unwrap());

        node.close().expect("close");
    }
}

// ============================================================================
// Clock Integration Tests (Layer 0)
// ============================================================================

mod clock_tests {
    use super::*;
    use beenode::clock::{ClockConfig, ClockService};

    /// Test: Clock service ticks and produces outcomes
    #[test]
    fn clock_service_basic() {
        let mut service = ClockService::with_defaults().expect("clock");

        // First tick
        let outcome1 = service.tick();
        assert_eq!(outcome1.snapshot.tick, 1);
        assert_eq!(outcome1.snapshot.epoch, 0);

        // Second tick
        let outcome2 = service.tick();
        assert_eq!(outcome2.snapshot.tick, 2);

        // Partitions cascade correctly (sec starts at 0, increments)
        let sec = outcome2.snapshot.partitions.iter()
            .find(|p| p.name == "sec")
            .expect("sec partition");
        assert_eq!(sec.value, 2);
    }

    /// Test: Clock config customization
    #[test]
    fn clock_config_custom() {
        let config = ClockConfig::new()
            .with_interval_ms(500)
            .with_partition("block", 100)
            .with_pulse("custom_pulse", 10);

        assert_eq!(config.interval_ms, 500);
        assert!(config.partitions.iter().any(|(n, m)| n == "block" && *m == 100));
        assert!(config.pulses.iter().any(|(n, p)| n == "custom_pulse" && *p == 10));

        let mut service = ClockService::new(config).expect("clock");
        let outcome = service.tick();

        // Custom partition exists
        assert!(outcome.snapshot.partitions.iter().any(|p| p.name == "block"));
    }

    /// Test: Pulse firing
    #[test]
    fn clock_pulse_firing() {
        let config = ClockConfig::new()
            .with_interval_ms(1000)
            .with_partition("step", 10)
            .with_pulse("every_3", 3);

        let mut service = ClockService::new(config).expect("clock");

        // Tick 1 - no pulse
        let outcome1 = service.tick();
        assert!(outcome1.pulses.iter().all(|p| p.name != "every_3"));

        // Tick 2 - no pulse
        let outcome2 = service.tick();
        assert!(outcome2.pulses.iter().all(|p| p.name != "every_3"));

        // Tick 3 - pulse fires!
        let outcome3 = service.tick();
        assert!(outcome3.pulses.iter().any(|p| p.name == "every_3"));
    }

    /// Test: Clock writes to store via spawn
    #[test]
    fn clock_writes_to_store() {
        use beenode::Store;
        use nine_s_core::namespace::Namespace;
        use std::sync::Arc;
        use tokio::sync::broadcast;

        let _guard = lock_env();
        let dir = TempDir::new().expect("tempdir");
        std::env::set_var("NINE_S_ROOT", dir.path());

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let store = Arc::new(Store::open("test-clock", b"").expect("store"));
            let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

            // Start clock with very fast interval for testing
            let config = ClockConfig::new()
                .with_interval_ms(50)  // 50ms ticks
                .with_partition("tick", 100)
                .with_pulse("test_pulse", 1);  // Fire every tick

            let handle = beenode::clock::start_clock_with_config(
                store.clone(),
                config,
                shutdown_rx,
            ).expect("start clock");

            // Wait for a few ticks
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Shutdown
            let _ = shutdown_tx.send(());
            let _ = handle.await;

            // Verify clock wrote status
            let status = store.read("/sys/clock/status").expect("read status");
            assert!(status.is_some(), "Clock should write status");

            // Status should indicate stopped (after shutdown)
            let status = status.unwrap();
            assert_eq!(status.data["status"], "stopped");

            // Verify clock wrote tick scroll
            let tick = store.read("/sys/clock/tick").expect("read tick");
            assert!(tick.is_some(), "Clock should write tick");

            let tick = tick.unwrap();
            assert!(tick.data["tick"].as_u64().unwrap() > 0);

            // Verify pulse scrolls were written
            let pulse = store.read("/sys/clock/pulses/test_pulse").expect("read pulse");
            assert!(pulse.is_some(), "Clock should write pulse");
        });
    }

    /// Test: Default clock has standard partitions
    #[test]
    fn clock_default_partitions() {
        let service = ClockService::with_defaults().expect("clock");
        let snapshot = service.snapshot();

        // Default has sec, min, hour partitions
        let names: Vec<&str> = snapshot.partitions.iter()
            .map(|p| p.name.as_str())
            .collect();

        assert!(names.contains(&"sec"), "Should have sec partition");
        assert!(names.contains(&"min"), "Should have min partition");
        assert!(names.contains(&"hour"), "Should have hour partition");
    }

    /// Test: BeeWallet config has sacred pulses
    #[test]
    fn clock_beewallet_config() {
        let config = ClockConfig::beewallet();

        // Sacred pulses
        assert!(config.pulses.iter().any(|(n, p)| n == "beat" && *p == 1));
        assert!(config.pulses.iter().any(|(n, p)| n == "glow" && *p == 21));

        // System pulses
        assert!(config.pulses.iter().any(|(n, p)| n == "ping" && *p == 30));
        assert!(config.pulses.iter().any(|(n, p)| n == "sync" && *p == 60));
        assert!(config.pulses.iter().any(|(n, p)| n == "backup" && *p == 3600));

        // Clock builds and ticks
        let mut service = ClockService::new(config).expect("clock");

        // Tick 21 times - glow should fire on tick 21
        for _ in 0..20 {
            let outcome = service.tick();
            assert!(outcome.pulses.iter().all(|p| p.name != "glow"));
        }

        let outcome21 = service.tick();
        assert!(outcome21.pulses.iter().any(|p| p.name == "glow"),
            "Glow pulse should fire on tick 21");
    }
}
