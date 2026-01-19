//! Mobi - Human-readable identifiers from secp256k1 public keys
//!
//! Mobi converts any secp256k1 public key to a unique 21-digit decimal number
//! using rejection sampling with SHA256 for uniform distribution.
//!
//! # Hierarchical Forms
//!
//! | Form | Digits | Collision at 50% |
//! |------|--------|------------------|
//! | Display | 12 | 1.4 million |
//! | Extended | 15 | 44.7 million |
//! | Long | 18 | 1.4 billion |
//! | Full | 21 | 44.7 billion |
//!
//! # Example
//!
//! ```ignore
//! let mobi = Mobi::derive("17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917")?;
//! assert_eq!(mobi.display, "879044656584");
//! assert_eq!(mobi.display_formatted(), "879-044-656-584");
//! ```

use nine_s_core::errors::{NineSError, NineSResult};
use sha2::{Digest, Sha256};

/// Human-readable 21-digit identifier derived from a secp256k1 public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mobi {
    /// 12 digits - shown to users (phone number style)
    pub display: String,
    /// 15 digits - for collision resolution
    pub extended: String,
    /// 18 digits - for higher uniqueness
    pub long: String,
    /// 21 digits - full form, stored internally
    pub full: String,
}

impl Mobi {
    /// Derive a Mobi from a secp256k1 public key (32 bytes hex).
    ///
    /// Uses rejection sampling to ensure uniform distribution:
    /// - Hash the pubkey (with round counter if needed)
    /// - Take first 9 bytes as 72-bit big-endian integer
    /// - If value < 10^21, use it; otherwise retry with incremented counter
    ///
    /// Expected iterations: ~4.7 rounds
    pub fn derive(pubkey_hex: &str) -> NineSResult<Self> {
        let pubkey_bytes = hex::decode(pubkey_hex)
            .map_err(|e| NineSError::Other(format!("Invalid hex pubkey: {}", e)))?;

        if pubkey_bytes.len() != 32 {
            return Err(NineSError::Other(format!(
                "Pubkey must be 32 bytes, got {}",
                pubkey_bytes.len()
            )));
        }

        // Rejection sampling: find value < 10^21
        for round in 0..=255u8 {
            let hash = if round == 0 {
                Sha256::digest(&pubkey_bytes)
            } else {
                let mut input = pubkey_bytes.clone();
                input.push(round);
                Sha256::digest(&input)
            };

            // Take first 9 bytes as 72-bit big-endian integer
            // We use u128 to hold the 72-bit value
            let value = u128::from_be_bytes([
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                hash[0],
                hash[1],
                hash[2],
                hash[3],
                hash[4],
                hash[5],
                hash[6],
                hash[7],
                hash[8],
            ]);

            // Check if value < 10^21
            const MAX_VALUE: u128 = 1_000_000_000_000_000_000_000;
            if value < MAX_VALUE {
                let full = format!("{:021}", value);
                return Ok(Self {
                    display: full[0..12].to_string(),
                    extended: full[0..15].to_string(),
                    long: full[0..18].to_string(),
                    full,
                });
            }
        }

        Err(NineSError::Other(
            "Mobi derivation failed after 256 rounds".into(),
        ))
    }

    /// Format display as phone number: "879-044-656-584"
    pub fn display_formatted(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            &self.display[0..3],
            &self.display[3..6],
            &self.display[6..9],
            &self.display[9..12]
        )
    }

    /// Format extended: "879-044-656-584-686"
    pub fn extended_formatted(&self) -> String {
        format!(
            "{}-{}-{}-{}-{}",
            &self.extended[0..3],
            &self.extended[3..6],
            &self.extended[6..9],
            &self.extended[9..12],
            &self.extended[12..15]
        )
    }

    /// Format full: "879-044-656-584-686-196-443"
    pub fn full_formatted(&self) -> String {
        format!(
            "{}-{}-{}-{}-{}-{}-{}",
            &self.full[0..3],
            &self.full[3..6],
            &self.full[6..9],
            &self.full[9..12],
            &self.full[12..15],
            &self.full[15..18],
            &self.full[18..21]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical test vectors from PROTOCOL.md
    #[test]
    fn test_canonical_vector_zeros() {
        let mobi = Mobi::derive("0000000000000000000000000000000000000000000000000000000000000000")
            .expect("derivation should succeed");
        assert_eq!(mobi.display, "587135537154");
        assert_eq!(mobi.full, "587135537154686717107");
    }

    #[test]
    fn test_canonical_vector_known() {
        let mobi = Mobi::derive("17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917")
            .expect("derivation should succeed");
        assert_eq!(mobi.display, "879044656584");
        assert_eq!(mobi.full, "879044656584686196443");
    }

    #[test]
    fn test_display_formatted() {
        let mobi = Mobi::derive("17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917")
            .expect("derivation should succeed");
        assert_eq!(mobi.display_formatted(), "879-044-656-584");
    }

    #[test]
    fn test_full_formatted() {
        let mobi = Mobi::derive("17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917")
            .expect("derivation should succeed");
        assert_eq!(mobi.full_formatted(), "879-044-656-584-686-196-443");
    }

    #[test]
    fn test_invalid_hex() {
        let result = Mobi::derive("not_valid_hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_length() {
        let result = Mobi::derive("1234"); // Too short
        assert!(result.is_err());
    }
}
