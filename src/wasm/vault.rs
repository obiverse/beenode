//! WASM Vault utilities for browser-based key management.
//!
//! This module exposes:
//! - BIP39 mnemonic generation/validation
//! - Seal/Unseal using PNIP-0011 (secret + time dimensions supported)

use bip39::{Language, Mnemonic};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use nine_s_core::scroll::Scroll;
use nine_s_store::seal::{self, Credentials, Seal};

#[derive(Debug, Deserialize)]
struct JsCredentials {
    password: Option<String>,
}

fn js_error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

fn from_js<T: for<'de> Deserialize<'de>>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|err| js_error(err))
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|err| js_error(err))
}

fn credentials_from_js(value: JsValue) -> Result<Credentials, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(Credentials::default());
    }
    let input: JsCredentials = from_js(value)?;
    Ok(match input.password {
        Some(password) => Credentials::with_password(password),
        None => Credentials::default(),
    })
}

/// WASM Vault API
#[wasm_bindgen]
pub struct WasmVault;

#[wasm_bindgen]
impl WasmVault {
    /// Generate a new BIP39 mnemonic (12 or 24 words).
    #[wasm_bindgen(js_name = "generateMnemonic")]
    pub fn generate_mnemonic(word_count: u8) -> Result<String, JsValue> {
        let entropy_len = match word_count {
            12 => 16,
            24 => 32,
            _ => return Err(js_error("word_count must be 12 or 24")),
        };

        let mut entropy = vec![0u8; entropy_len];
        rand::thread_rng().fill_bytes(&mut entropy);

        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|err| js_error(format!("mnemonic generation failed: {err}")))?;

        Ok(mnemonic.to_string())
    }

    /// Validate a BIP39 mnemonic (English wordlist).
    #[wasm_bindgen(js_name = "validateMnemonic")]
    pub fn validate_mnemonic(mnemonic: &str) -> bool {
        Mnemonic::parse_in(Language::English, mnemonic).is_ok()
    }

    /// Seal a scroll with a secret (password/PIN).
    ///
    /// Returns a sealed scroll (type: 9s/sealed@v1).
    #[wasm_bindgen(js_name = "sealWithPassword")]
    pub fn seal_with_password(scroll_json: JsValue, password: &str) -> Result<JsValue, JsValue> {
        let scroll: Scroll = from_js(scroll_json)?;
        let seal_config = Seal::with_secret(password).map_err(js_error)?;
        let credentials = Credentials::with_password(password);
        let sealed = seal::seal(&scroll, &seal_config, &credentials).map_err(js_error)?;
        to_js(&sealed)
    }

    /// Seal a scroll with secret + expiry (PreventNow pattern).
    ///
    /// Both the password AND the expiry window must be satisfied to unseal.
    #[wasm_bindgen(js_name = "sealWithPasswordAndExpiry")]
    pub fn seal_with_password_and_expiry(
        scroll_json: JsValue,
        password: &str,
        expires_at: &str,
    ) -> Result<JsValue, JsValue> {
        let scroll: Scroll = from_js(scroll_json)?;
        let seal_config = Seal::preventnow_default(password, expires_at).map_err(js_error)?;
        let credentials = Credentials::with_password(password);
        let sealed = seal::seal(&scroll, &seal_config, &credentials).map_err(js_error)?;
        to_js(&sealed)
    }

    /// Seal a scroll with a full Seal config and optional credentials.
    ///
    /// `seal_config` must include any secret kdf/verifier fields needed.
    #[wasm_bindgen(js_name = "sealWithConfig")]
    pub fn seal_with_config(
        scroll_json: JsValue,
        seal_config: JsValue,
        credentials: JsValue,
    ) -> Result<JsValue, JsValue> {
        let scroll: Scroll = from_js(scroll_json)?;
        let seal_config: Seal = from_js(seal_config)?;
        let credentials = credentials_from_js(credentials)?;
        let sealed = seal::seal(&scroll, &seal_config, &credentials).map_err(js_error)?;
        to_js(&sealed)
    }

    /// Unseal a sealed scroll using a password.
    #[wasm_bindgen(js_name = "unsealWithPassword")]
    pub fn unseal_with_password(sealed_json: JsValue, password: &str) -> Result<JsValue, JsValue> {
        let sealed: Scroll = from_js(sealed_json)?;
        let credentials = Credentials::with_password(password);
        let unsealed = seal::unseal(&sealed, &credentials).map_err(js_error)?;
        to_js(&unsealed)
    }

    /// Unseal a sealed scroll using credentials.
    #[wasm_bindgen(js_name = "unseal")]
    pub fn unseal(sealed_json: JsValue, credentials: JsValue) -> Result<JsValue, JsValue> {
        let sealed: Scroll = from_js(sealed_json)?;
        let credentials = credentials_from_js(credentials)?;
        let unsealed = seal::unseal(&sealed, &credentials).map_err(js_error)?;
        to_js(&unsealed)
    }
}
