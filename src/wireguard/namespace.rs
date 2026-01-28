//! WireGuard Namespace
//!
//! Provides scroll-based access to WireGuard identity and configuration.
//!
//! ## Paths
//!
//! | Path | R/W | Description |
//! |------|-----|-------------|
//! | `/wireguard/status` | R | `{ initialized: bool }` |
//! | `/wireguard/pubkey` | R | `{ base64: "...", hex: "..." }` |
//! | `/wireguard/config` | W | Write server config → returns client config |

use super::{public_key_to_base64, WireGuardConfig, WireGuardKeypair};
use nine_s_core::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;

/// WireGuard namespace for scroll-based access
pub struct WireGuardNamespace {
    keypair: Arc<WireGuardKeypair>,
    config: Option<WireGuardConfig>,
}

impl WireGuardNamespace {
    /// Create a new WireGuard namespace with the given keypair
    pub fn new(keypair: WireGuardKeypair) -> Self {
        Self {
            keypair: Arc::new(keypair),
            config: None,
        }
    }

    /// Create with pre-configured tunnel settings
    pub fn with_config(keypair: WireGuardKeypair, config: WireGuardConfig) -> Self {
        Self {
            keypair: Arc::new(keypair),
            config: Some(config),
        }
    }

    fn read_status(&self) -> Scroll {
        Scroll::typed(
            "/wireguard/status",
            json!({
                "initialized": true,
                "has_config": self.config.is_some(),
            }),
            "wireguard/status@v1",
        )
    }

    fn read_pubkey(&self) -> Scroll {
        let b64 = public_key_to_base64(&self.keypair.public_key);
        let hex_str = hex::encode(&self.keypair.public_key);

        Scroll::typed(
            "/wireguard/pubkey",
            json!({
                "base64": b64,
                "hex": hex_str,
            }),
            "wireguard/pubkey@v1",
        )
    }

    fn read_config(&self) -> Option<Scroll> {
        self.config.as_ref().map(|cfg| {
            Scroll::typed(
                "/wireguard/config",
                json!({
                    "config_file": cfg.to_config_string(),
                    "server_endpoint": cfg.server_endpoint,
                    "tunnel_address": cfg.tunnel_address,
                    "server_pubkey": public_key_to_base64(&cfg.server_public_key),
                }),
                "wireguard/config@v1",
            )
        })
    }
}

impl Namespace for WireGuardNamespace {
    fn read(&self, path: &str) -> NineSResult<Option<Scroll>> {
        match path {
            "status" | "/status" => Ok(Some(self.read_status())),
            "pubkey" | "/pubkey" => Ok(Some(self.read_pubkey())),
            "config" | "/config" => Ok(self.read_config()),
            _ => Ok(None),
        }
    }

    fn write(&self, path: &str, _data: Value) -> NineSResult<Scroll> {
        // For now, config is set at construction time
        // Future: allow dynamic config updates via write
        match path {
            "config" | "/config" => {
                // Return current config or error
                self.read_config()
                    .ok_or_else(|| NineSError::Other("No WireGuard config set".into()))
            }
            _ => Err(NineSError::invalid_path(path, "unknown wireguard path")),
        }
    }

    fn list(&self, _prefix: &str) -> NineSResult<Vec<String>> {
        let mut paths = vec![
            "/wireguard/status".to_string(),
            "/wireguard/pubkey".to_string(),
        ];
        if self.config.is_some() {
            paths.push("/wireguard/config".to_string());
        }
        Ok(paths)
    }

    fn close(&self) -> NineSResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wireguard::derive_keypair;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_namespace_status() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let ns = WireGuardNamespace::new(keypair);

        let scroll = ns.read("status").unwrap().unwrap();
        assert_eq!(scroll.data["initialized"], true);
        assert_eq!(scroll.data["has_config"], false);
    }

    #[test]
    fn test_namespace_pubkey() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let ns = WireGuardNamespace::new(keypair);

        let scroll = ns.read("pubkey").unwrap().unwrap();
        assert!(scroll.data["base64"].as_str().unwrap().len() == 44);
        assert!(scroll.data["hex"].as_str().unwrap().len() == 64);
    }

    #[test]
    fn test_namespace_config() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let config = WireGuardConfig {
            private_key: keypair.private_key,
            server_public_key: [0x42u8; 32],
            server_endpoint: "wg.example.com:51820".into(),
            tunnel_address: "10.21.0.1/32".into(),
            dns: Some(vec!["1.1.1.1".into()]),
            persistent_keepalive: 21,
        };
        let ns = WireGuardNamespace::with_config(keypair, config);

        let scroll = ns.read("config").unwrap().unwrap();
        assert!(scroll.data["config_file"]
            .as_str()
            .unwrap()
            .contains("[Interface]"));
        assert_eq!(
            scroll.data["server_endpoint"].as_str().unwrap(),
            "wg.example.com:51820"
        );
    }

    #[test]
    fn test_namespace_list() {
        let keypair = derive_keypair(TEST_MNEMONIC, None).unwrap();
        let ns = WireGuardNamespace::new(keypair);

        let paths = ns.list("").unwrap();
        assert!(paths.contains(&"/wireguard/status".to_string()));
        assert!(paths.contains(&"/wireguard/pubkey".to_string()));
    }
}
