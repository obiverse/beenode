//! PIN-based authentication and mnemonic encryption.

use nine_s_core::errors::{NineSError, NineSResult};
use nine_s_store::crypto::{
    decrypt_with_aad, derive_key_from_password, encrypt_with_aad, generate_argon2_salt, DerivedKey,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AAD_MNEMONIC: &[u8] = b"beenode-mnemonic";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthFile {
    salt: String,
    verifier: String,
    encrypted_mnemonic: String,
    nonce: String,
}

#[derive(Debug, Clone)]
pub struct PinAuth {
    path: PathBuf,
    data: Option<AuthFile>,
}

impl PinAuth {
    pub fn load(app: &str) -> NineSResult<Self> {
        let path = auth_path(app)?;
        let data = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| NineSError::Other(format!("auth read: {e}")))?;
            Some(serde_json::from_str(&raw)
                .map_err(|e| NineSError::Other(format!("auth json: {e}")))?)
        } else {
            None
        };
        Ok(Self { path, data })
    }

    pub fn is_initialized(&self) -> bool { self.data.is_some() }

    pub fn verify_pin(&self, pin: &str) -> NineSResult<bool> {
        let data = self.data.as_ref().ok_or_else(|| NineSError::Other("auth not initialized".into()))?;
        let key = Self::derive_key(pin, &decode_base64(&data.salt)?)?;
        let verifier = blake3::hash(&key.0).to_hex().to_string();
        Ok(verifier == data.verifier)
    }

    pub fn set_pin(&mut self, pin: &str, mnemonic: &str) -> NineSResult<()> {
        let encrypted = self.encrypt_mnemonic(mnemonic, pin)?;
        let data = AuthFile {
            salt: encode_base64(&encrypted.salt),
            verifier: encrypted.verifier,
            encrypted_mnemonic: encode_base64(&encrypted.ciphertext),
            nonce: encode_base64(&encrypted.nonce),
        };
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NineSError::Other(format!("auth mkdir: {e}")))?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(&data).unwrap())
            .map_err(|e| NineSError::Other(format!("auth write: {e}")))?;
        self.data = Some(data);
        Ok(())
    }

    pub fn derive_key(pin: &str, salt: &[u8]) -> NineSResult<DerivedKey> {
        derive_key_from_password(pin.as_bytes(), salt)
    }

    pub fn encrypt_mnemonic(&self, mnemonic: &str, pin: &str) -> NineSResult<EncryptedMnemonic> {
        let salt = generate_argon2_salt();
        let key = Self::derive_key(pin, &salt)?;
        let verifier = blake3::hash(&key.0).to_hex().to_string();
        let (nonce, ciphertext) = encrypt_with_aad(&key, mnemonic.as_bytes(), AAD_MNEMONIC)?;
        Ok(EncryptedMnemonic { salt, verifier, nonce, ciphertext })
    }

    pub fn decrypt_mnemonic(&self, pin: &str) -> NineSResult<String> {
        let data = self.data.as_ref().ok_or_else(|| NineSError::Other("auth not initialized".into()))?;
        let salt = decode_base64(&data.salt)?;
        let nonce = decode_base64(&data.nonce)?;
        let ciphertext = decode_base64(&data.encrypted_mnemonic)?;
        let key = Self::derive_key(pin, &salt)?;
        let nonce: [u8; 12] = nonce
            .try_into()
            .map_err(|_| NineSError::Other("auth nonce invalid".into()))?;
        let plaintext = decrypt_with_aad(&key, &nonce, &ciphertext, AAD_MNEMONIC)?;
        String::from_utf8(plaintext).map_err(|e| NineSError::Other(format!("mnemonic utf8: {e}")))
    }
}

pub struct EncryptedMnemonic {
    pub salt: [u8; 16],
    pub verifier: String,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

fn auth_path(app: &str) -> NineSResult<PathBuf> {
    let root = std::env::var("NINE_S_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")));
    Ok(root.join(app).join("data").join("auth.json"))
}

fn encode_base64(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn decode_base64(value: &str) -> NineSResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|e| NineSError::Other(format!("base64: {e}")))
}
