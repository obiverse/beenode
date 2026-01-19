//! Identity - Derives keys from seed. Master mnemonic NEVER leaves layer 0.

mod bip85;

use crate::mobi::Mobi;
use nine_s_core::errors::{NineSError, NineSResult};

pub use bip85::{derive_nostr_mnemonic, Bip85Error};

#[derive(Debug, Clone)]
pub struct Identity {
    #[cfg(feature = "nostr")] pub nostr_keys: nostr::Keys,
    pub mobi: Mobi,
    pub pubkey_hex: String,
}

impl Identity {
    #[cfg(feature = "nostr")]
    pub fn from_seed(seed: &[u8; 64]) -> NineSResult<Self> {
        let sk = nostr::SecretKey::from_slice(&seed[..32]).map_err(|e| NineSError::Other(e.to_string()))?;
        let keys = nostr::Keys::new(sk);
        let pubkey_hex = keys.public_key().to_hex();
        Ok(Self { nostr_keys: keys, mobi: Mobi::derive(&pubkey_hex)?, pubkey_hex })
    }

    #[cfg(not(feature = "nostr"))]
    pub fn from_seed(seed: &[u8; 64]) -> NineSResult<Self> {
        use bitcoin::secp256k1::{Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&seed[..32]).map_err(|e| NineSError::Other(e.to_string()))?;
        let pubkey_hex = hex::encode(&sk.public_key(&secp).x_only_public_key().0.serialize());
        Ok(Self { mobi: Mobi::derive(&pubkey_hex)?, pubkey_hex })
    }

    #[cfg(feature = "nostr")]
    pub fn from_mnemonic(mnemonic_str: &str) -> NineSResult<Self> {
        let nostr_mnemonic = derive_nostr_mnemonic(mnemonic_str, None).map_err(|e| NineSError::Other(e.to_string()))?;
        let m = bip39::Mnemonic::parse(&nostr_mnemonic).map_err(|e| NineSError::Other(e.to_string()))?;
        let sk = nostr::SecretKey::from_slice(&m.to_seed("")[..32]).map_err(|e| NineSError::Other(e.to_string()))?;
        let keys = nostr::Keys::new(sk);
        let pubkey_hex = keys.public_key().to_hex();
        Ok(Self { nostr_keys: keys, mobi: Mobi::derive(&pubkey_hex)?, pubkey_hex })
    }

    #[cfg(not(feature = "nostr"))]
    pub fn from_mnemonic(mnemonic_str: &str) -> NineSResult<Self> {
        use bitcoin::secp256k1::{Secp256k1, SecretKey};
        let nostr_mnemonic = derive_nostr_mnemonic(mnemonic_str, None).map_err(|e| NineSError::Other(e.to_string()))?;
        let m = bip39::Mnemonic::parse(&nostr_mnemonic).map_err(|e| NineSError::Other(e.to_string()))?;
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&m.to_seed("")[..32]).map_err(|e| NineSError::Other(e.to_string()))?;
        let pubkey_hex = hex::encode(&sk.public_key(&secp).x_only_public_key().0.serialize());
        Ok(Self { mobi: Mobi::derive(&pubkey_hex)?, pubkey_hex })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_identity_from_mnemonic() {
        let identity = Identity::from_mnemonic(TEST_MNEMONIC).expect("should derive");
        assert_eq!(identity.pubkey_hex.len(), 64);
        assert_eq!(identity.mobi.display.len(), 12);
    }

    #[test]
    fn test_identity_from_seed() {
        // Simulate a 64-byte seed from keychain
        let mut seed = [0u8; 64];
        seed[..32].copy_from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ]);

        let identity = Identity::from_seed(&seed).expect("should derive");
        assert_eq!(identity.pubkey_hex.len(), 64);
        assert_eq!(identity.mobi.display.len(), 12);
    }

    #[test]
    fn test_identity_deterministic() {
        let id1 = Identity::from_mnemonic(TEST_MNEMONIC).expect("should derive");
        let id2 = Identity::from_mnemonic(TEST_MNEMONIC).expect("should derive");
        assert_eq!(id1.pubkey_hex, id2.pubkey_hex);
        assert_eq!(id1.mobi.full, id2.mobi.full);
    }
}
