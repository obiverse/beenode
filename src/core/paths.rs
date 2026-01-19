//! Path and Type constants for 9S namespaces
//!
//! Centralized registry for all paths and scroll types.
//! Use enum variants for type-safe path matching.

/// Wallet paths
pub mod wallet {
    pub const STATUS: &str = "/status";
    pub const BALANCE: &str = "/balance";
    pub const ADDRESS: &str = "/address";
    pub const NETWORK: &str = "/network";
    pub const TRANSACTIONS: &str = "/transactions";
    pub const SYNC: &str = "/sync";
    pub const SEND: &str = "/send";
    pub const RECEIVE: &str = "/receive";
    pub const FEE_ESTIMATE: &str = "/fee-estimate";
    pub const UTXOS: &str = "/utxos";

    pub const EXTERNAL_SYNC: &str = "/external/bitcoin/sync";
    pub const EXTERNAL_SEND: &str = "/external/bitcoin/send";

    pub const ALL: &[&str] = &[STATUS, BALANCE, ADDRESS, NETWORK, TRANSACTIONS, RECEIVE, UTXOS];
}

/// Nostr paths
pub mod nostr {
    pub const STATUS: &str = "/status";
    pub const PUBKEY: &str = "/pubkey";
    pub const MOBI: &str = "/mobi";
    pub const RELAYS: &str = "/relays";
    pub const SIGN: &str = "/sign";
    pub const CONNECT: &str = "/connect";
    pub const PUBLISH: &str = "/publish";

    pub const EXTERNAL_CONNECT: &str = "/external/nostr/connect";
    pub const EXTERNAL_PUBLISH: &str = "/external/nostr/publish";

    pub const ALL: &[&str] = &[STATUS, PUBKEY, MOBI, RELAYS];
}

/// Nostr scroll types
pub mod nostr_types {
    pub const STATUS: &str = "nostr/status@v1";
    pub const PUBKEY: &str = "nostr/pubkey@v1";
    pub const MOBI: &str = "nostr/mobi@v1";
    pub const RELAYS: &str = "nostr/relays@v1";
    pub const SIGNATURE: &str = "nostr/signature@v1";
    pub const CONNECT: &str = "nostr/connect@v1";
    pub const PUBLISH: &str = "nostr/publish@v1";
}

/// Clock paths (Layer 0)
pub mod clock {
    pub const STATUS: &str = "/sys/clock/status";
    pub const TICK: &str = "/sys/clock/tick";
    pub const PULSES: &str = "/sys/clock/pulses";
    pub const CONFIG: &str = "/sys/clock/config";

    pub const TICK_TYPE: &str = "clock/tick@v1";
    pub const PULSE_TYPE: &str = "clock/pulse@v1";
    pub const STATUS_TYPE: &str = "clock/status@v1";
}

/// Mind/Effects paths
pub mod mind {
    pub const PATTERNS_PREFIX: &str = "/sys/mind/patterns";
    pub const EXTERNAL_PREFIX: &str = "/external";
    pub const RESERVED_SUFFIX: &str = "/_init";
    pub const RESULT_SUFFIX: &str = "/result";
}

/// Scroll type for effect results
pub const EFFECT_RESULT_TYPE: &str = "effect/result@v1";

/// Origin markers for loop prevention
pub mod origin {
    pub const CLOCK: &str = "clock";
    pub const MIND: &str = "mind";
    pub const EFFECTS: &str = "effects";
}
