//! BIP85 - Deterministic Entropy from BIP32 Keychains
//!
//! Derives child mnemonics from a master seed so that subsystems (Lightning, Nostr)
//! get isolated keys. If one subsystem is compromised, others remain safe.
//!
//! Reference: https://bips.xyz/85

use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::secp256k1::Secp256k1;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::str::FromStr;

/// BIP85 application indices
#[allow(dead_code)]
pub const INDEX_LIGHTNING: u32 = 0;
pub const INDEX_NOSTR: u32 = 1;

/// Errors during BIP85 derivation
#[derive(Debug, thiserror::Error)]
pub enum Bip85Error {
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("Derivation failed: {0}")]
    DerivationFailed(String),
    #[error("Invalid word count: {0}")]
    InvalidWordCount(u32),
}

/// Derive a child mnemonic using BIP85
///
/// # Arguments
/// * `master_mnemonic` - The master BIP39 mnemonic (12 or 24 words)
/// * `passphrase` - Optional BIP39 passphrase for the master seed
/// * `words` - Number of words for child mnemonic (12 or 24)
/// * `index` - Application index (0 = Lightning, 1 = Nostr, etc.)
pub fn derive_mnemonic(
    master_mnemonic: &str,
    passphrase: Option<&str>,
    words: u32,
    index: u32,
) -> Result<String, Bip85Error> {
    // Validate word count
    let entropy_bits = match words {
        12 => 128,
        24 => 256,
        _ => return Err(Bip85Error::InvalidWordCount(words)),
    };
    let entropy_bytes = entropy_bits / 8;

    // Parse master mnemonic
    let mnemonic = Mnemonic::parse_normalized(master_mnemonic)
        .map_err(|e| Bip85Error::InvalidMnemonic(e.to_string()))?;

    // Derive master seed
    let seed = mnemonic.to_seed(passphrase.unwrap_or(""));

    // Create master xpriv
    let secp = Secp256k1::new();
    let master_xpriv = Xpriv::new_master(bitcoin::Network::Bitcoin, &seed)
        .map_err(|e| Bip85Error::DerivationFailed(e.to_string()))?;

    // BIP85 derivation path: m/83696968'/39'/0'/{words}'/{index}'
    // 83696968 = 0x50524231 = "BIP85" in ASCII
    // 39 = BIP39 application
    // 0 = English language
    let path_str = format!("m/83696968'/39'/0'/{}'/{}'" , words, index);
    let path = DerivationPath::from_str(&path_str)
        .map_err(|e| Bip85Error::DerivationFailed(e.to_string()))?;

    // Derive child key
    let derived = master_xpriv
        .derive_priv(&secp, &path)
        .map_err(|e| Bip85Error::DerivationFailed(e.to_string()))?;

    // HMAC-SHA512 with "bip-entropy-from-k" as per BIP85
    let mut hmac = Hmac::<Sha512>::new_from_slice(b"bip-entropy-from-k")
        .expect("HMAC accepts any key length");
    hmac.update(&derived.private_key.secret_bytes());
    let result = hmac.finalize().into_bytes();

    // Take required entropy bytes
    let entropy = &result[..entropy_bytes];

    // Create child mnemonic
    let child_mnemonic = Mnemonic::from_entropy(entropy)
        .map_err(|e| Bip85Error::DerivationFailed(e.to_string()))?;

    Ok(child_mnemonic.to_string())
}

/// Derive Nostr mnemonic (index 1, 12 words)
pub fn derive_nostr_mnemonic(
    master_mnemonic: &str,
    passphrase: Option<&str>,
) -> Result<String, Bip85Error> {
    derive_mnemonic(master_mnemonic, passphrase, 12, INDEX_NOSTR)
}

/// Derive Lightning mnemonic (index 0, 12 words)
#[allow(dead_code)]
pub fn derive_lightning_mnemonic(
    master_mnemonic: &str,
    passphrase: Option<&str>,
) -> Result<String, Bip85Error> {
    derive_mnemonic(master_mnemonic, passphrase, 12, INDEX_LIGHTNING)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_derive_12_word_mnemonic() {
        let child = derive_mnemonic(TEST_MNEMONIC, None, 12, 0).unwrap();
        let words: Vec<&str> = child.split_whitespace().collect();
        assert_eq!(words.len(), 12);
        assert!(Mnemonic::parse_normalized(&child).is_ok());
    }

    #[test]
    fn test_deterministic_derivation() {
        let child1 = derive_mnemonic(TEST_MNEMONIC, None, 12, 0).unwrap();
        let child2 = derive_mnemonic(TEST_MNEMONIC, None, 12, 0).unwrap();
        assert_eq!(child1, child2);
    }

    #[test]
    fn test_different_indices() {
        let lightning = derive_lightning_mnemonic(TEST_MNEMONIC, None).unwrap();
        let nostr = derive_nostr_mnemonic(TEST_MNEMONIC, None).unwrap();
        assert_ne!(lightning, nostr);
    }
}
