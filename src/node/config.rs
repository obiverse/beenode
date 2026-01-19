//! Node Configuration - passed from higher layers

use crate::core::pattern::PatternDef;
#[cfg(feature = "wallet")]
use crate::wallet::Network;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Pin,
    None,
}

impl Default for AuthMode {
    fn default() -> Self { Self::Pin }
}

impl AuthMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthMode::Pin => "pin",
            AuthMode::None => "none",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pin" => Some(AuthMode::Pin),
            "none" | "disabled" | "off" => Some(AuthMode::None),
            _ => None,
        }
    }
}

/// Node configuration. Higher layers construct this.
#[derive(Debug, Clone, Default)]
pub struct NodeConfig {
    pub app: String,
    pub master_key: Vec<u8>,
    pub mnemonic: Option<String>,
    pub auth_mode: AuthMode,
    #[cfg(feature = "wallet")]
    pub wallet: Option<WalletConfig>,
    #[cfg(feature = "nostr")]
    pub nostr: Option<NostrConfig>,
    pub enable_mind: bool,
    pub patterns: Vec<PatternDef>,
}

impl NodeConfig {
    pub fn new(app: impl Into<String>) -> Self {
        Self { app: app.into(), ..Default::default() }
    }
    pub fn with_master_key(mut self, key: Vec<u8>) -> Self { self.master_key = key; self }
    pub fn with_mnemonic(mut self, m: impl Into<String>) -> Self { self.mnemonic = Some(m.into()); self }
    pub fn with_auth_mode(mut self, mode: AuthMode) -> Self { self.auth_mode = mode; self }
    #[cfg(feature = "wallet")]
    pub fn with_wallet(mut self, c: WalletConfig) -> Self { self.wallet = Some(c); self }
    #[cfg(feature = "nostr")]
    pub fn with_nostr(mut self, c: NostrConfig) -> Self { self.nostr = Some(c); self }
    pub fn with_mind(mut self, patterns: Vec<PatternDef>) -> Self { self.enable_mind = true; self.patterns = patterns; self }
}

#[cfg(feature = "wallet")]
#[derive(Debug, Clone)]
pub struct WalletConfig {
    pub network: Network,
    pub electrum_url: Option<String>,
    pub data_dir: Option<std::path::PathBuf>,
    /// Bitcoin RPC config (for regtest/Polar testing)
    #[cfg(feature = "bitcoind-rpc")]
    pub rpc: Option<RpcConfig>,
}

#[cfg(feature = "wallet")]
impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            network: Network::default(),
            electrum_url: None,
            data_dir: None,
            #[cfg(feature = "bitcoind-rpc")]
            rpc: None,
        }
    }
}

/// Bitcoin Core RPC configuration
#[cfg(feature = "bitcoind-rpc")]
#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub url: String,
    pub user: String,
    pub pass: String,
}

#[cfg(feature = "wallet")]
impl WalletConfig {
    pub fn mainnet() -> Self { Self { network: Network::Bitcoin, electrum_url: None, data_dir: None, #[cfg(feature = "bitcoind-rpc")] rpc: None } }
    pub fn testnet() -> Self { Self { network: Network::Testnet, electrum_url: None, data_dir: None, #[cfg(feature = "bitcoind-rpc")] rpc: None } }
    pub fn with_electrum(mut self, url: impl Into<String>) -> Self { self.electrum_url = Some(url.into()); self }
    pub fn with_data_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self { self.data_dir = Some(path.into()); self }
    #[cfg(feature = "bitcoind-rpc")]
    pub fn with_rpc(mut self, url: impl Into<String>, user: impl Into<String>, pass: impl Into<String>) -> Self {
        self.rpc = Some(RpcConfig { url: url.into(), user: user.into(), pass: pass.into() });
        self
    }
}

#[cfg(feature = "nostr")]
#[derive(Debug, Clone)]
pub struct NostrConfig {
    pub relays: Vec<String>,
    pub beebase_url: Option<String>,
    pub auto_connect: bool,
}

#[cfg(feature = "nostr")]
impl Default for NostrConfig {
    fn default() -> Self { Self { relays: vec!["wss://relay.damus.io".into()], beebase_url: None, auto_connect: false } }
}

#[cfg(feature = "nostr")]
impl NostrConfig {
    pub fn with_relays(relays: Vec<String>) -> Self { Self { relays, ..Default::default() } }
    pub fn with_beebase(mut self, url: impl Into<String>) -> Self { self.beebase_url = Some(url.into()); self }
    pub fn auto_connect(mut self) -> Self { self.auto_connect = true; self }
}
