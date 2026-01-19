//! WalletNamespace - Bitcoin wallet via 9S paths. Writes to /external/* trigger effects.

use crate::core::paths::wallet as paths;
use nine_s_core::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[cfg(feature = "wallet")]
use crate::wallet::bdk::BdkWallet;
#[cfg(feature = "wallet")]
use nine_s_store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Network { #[default] Bitcoin, Testnet, Signet, Regtest }

impl Network {
    pub fn as_str(&self) -> &'static str {
        match self { Network::Bitcoin => "bitcoin", Network::Testnet => "testnet", Network::Signet => "signet", Network::Regtest => "regtest" }
    }
    #[cfg(feature = "wallet")]
    pub fn to_bdk(&self) -> bdk_wallet::bitcoin::Network {
        match self { Network::Bitcoin => bdk_wallet::bitcoin::Network::Bitcoin, Network::Testnet => bdk_wallet::bitcoin::Network::Testnet, Network::Signet => bdk_wallet::bitcoin::Network::Signet, Network::Regtest => bdk_wallet::bitcoin::Network::Regtest }
    }
}

#[cfg(feature = "wallet")]
pub struct WalletNamespace { wallet: Arc<BdkWallet>, store: Arc<Store>, network: Network }

#[cfg(feature = "wallet")]
impl WalletNamespace {
    pub fn open(seed: &[u8; 64], store: Arc<Store>, network: Network, db_path: &std::path::Path, electrum_url: Option<&str>) -> NineSResult<Self> {
        Ok(Self { wallet: Arc::new(BdkWallet::open(seed, network.to_bdk(), db_path, electrum_url)?), store, network })
    }

    #[cfg(feature = "bitcoind-rpc")]
    pub fn open_rpc(seed: &[u8; 64], store: Arc<Store>, network: Network, db_path: &std::path::Path, rpc_url: &str, rpc_user: &str, rpc_pass: &str) -> NineSResult<Self> {
        Ok(Self { wallet: Arc::new(BdkWallet::open_rpc(seed, network.to_bdk(), db_path, rpc_url, rpc_user, rpc_pass)?), store, network })
    }

    pub fn wallet_handle(&self) -> Arc<BdkWallet> { self.wallet.clone() }
}

#[cfg(feature = "wallet")]
impl Namespace for WalletNamespace {
    fn read(&self, path: &str) -> NineSResult<Option<Scroll>> {
        Ok(Some(match path {
            paths::STATUS | "" | "/" => Scroll::new("/wallet/status", json!({"initialized": true, "network": self.network.as_str()})),
            paths::BALANCE => {
                let b = self.wallet.balance()?;
                let pending = b.trusted_pending + b.untrusted_pending;
                let total = b.confirmed + pending;
                Scroll::new(
                    "/wallet/balance",
                    json!({
                        "confirmed": b.confirmed,
                        "pending": pending,
                        "immature": b.immature,
                        "spendable": b.confirmed,
                        "total": total
                    }),
                )
            }
            paths::ADDRESS => Scroll::new("/wallet/address", json!({"address": self.wallet.receive_address()?})),
            paths::NETWORK => Scroll::new("/wallet/network", json!({"network": self.network.as_str()})),
            paths::TRANSACTIONS => {
                let txs = self.wallet.transactions(50)?;
                Scroll::new(
                    "/wallet/transactions",
                    json!({
                        "transactions": txs.iter().map(|tx| json!({
                            "txid": tx.txid,
                            "received": tx.received,
                            "sent": tx.sent,
                            "fee": tx.fee,
                            "confirmed": tx.confirmed,
                            "is_confirmed": tx.confirmed,
                            "timestamp": tx.timestamp,
                            "block_height": tx.block_height
                        })).collect::<Vec<_>>(),
                        "count": txs.len()
                    }),
                )
            }
            paths::UTXOS => { let utxos = self.wallet.list_unspent()?; let total: u64 = utxos.iter().map(|u| u.amount_sat).sum(); Scroll::new("/wallet/utxos", json!({"utxos": utxos.iter().map(|u| json!({"txid": u.txid, "vout": u.vout, "amount_sat": u.amount_sat, "address": u.address, "is_change": u.is_change})).collect::<Vec<_>>(), "count": utxos.len(), "total_sat": total})) }
            _ => return Ok(None),
        }))
    }

    fn write(&self, path: &str, data: Value) -> NineSResult<Scroll> {
        let id = uuid();
        match path {
            paths::ADDRESS => {
                let new_requested = data
                    .get("new")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let address = if new_requested {
                    self.wallet.new_address()?
                } else {
                    self.wallet.receive_address()?
                };
                Ok(Scroll::new("/wallet/address", json!({"address": address})))
            }
            paths::RECEIVE => {
                let address = self.wallet.receive_address()?;
                let amount_sat = data.get("amount_sat")
                    .and_then(|v| v.as_u64())
                    .or_else(|| data.get("amount").and_then(|v| v.as_u64()));
                let label = data.get("label")
                    .and_then(|v| v.as_str())
                    .or_else(|| data.get("description").and_then(|v| v.as_str()));
                let message = data.get("message").and_then(|v| v.as_str());

                let mut uri = format!("bitcoin:{}", address);
                let mut query = Vec::new();
                if let Some(amount) = amount_sat {
                    query.push(format!("amount={}", format_btc_amount(amount)));
                }
                if let Some(label) = label {
                    query.push(format!("label={}", percent_encode(label)));
                }
                if let Some(message) = message {
                    query.push(format!("message={}", percent_encode(message)));
                }
                if !query.is_empty() {
                    uri.push('?');
                    uri.push_str(&query.join("&"));
                }

                Ok(Scroll::new(
                    "/wallet/receive",
                    json!({
                        "address": address,
                        "uri": uri,
                        "amount_sat": amount_sat,
                        "label": label,
                        "message": message
                    }),
                ))
            }
            paths::SYNC => {
                // Sync now if requested, else queue to effects
                if data.get("now").and_then(|v| v.as_bool()).unwrap_or(true) {
                    self.wallet.sync()?;
                    let b = self.wallet.balance()?;
                    Ok(Scroll::new("/wallet/sync", json!({"status": "synced", "confirmed": b.confirmed, "pending": b.trusted_pending + b.untrusted_pending})))
                } else {
                    self.store.write_scroll(Scroll::new(&format!("{}/{}", paths::EXTERNAL_SYNC, id), json!({"network": self.network.as_str()})))?;
                    Ok(Scroll::new("/wallet/sync", json!({"status": "pending", "request_id": id})))
                }
            }
            paths::SEND => {
                let to = data["to"].as_str().ok_or_else(|| NineSError::Other("no 'to'".into()))?;
                let amt = data.get("amount_sat")
                    .and_then(|v| v.as_u64())
                    .or_else(|| data.get("amount").and_then(|v| v.as_u64()))
                    .ok_or_else(|| NineSError::Other("no 'amount_sat'".into()))?;
                let fee_rate = data["fee_rate"].as_f64();
                // Execute now by default, queue to effects if now=false
                if data.get("now").and_then(|v| v.as_bool()).unwrap_or(true) {
                    let txid = self.wallet.send(to, amt, fee_rate)?;
                    Ok(Scroll::new("/wallet/send", json!({"status": "broadcast", "txid": txid, "to": to, "amount_sat": amt})))
                } else {
                    self.store.write_scroll(Scroll::new(&format!("{}/{}", paths::EXTERNAL_SEND, id), json!({"to": to, "amount_sat": amt, "fee_rate": fee_rate})))?;
                    Ok(Scroll::new("/wallet/send", json!({"status": "pending", "request_id": id, "to": to, "amount_sat": amt})))
                }
            }
            paths::FEE_ESTIMATE => {
                let to = data["to"].as_str().ok_or_else(|| NineSError::Other("no 'to'".into()))?;
                let amt = data.get("amount_sat")
                    .and_then(|v| v.as_u64())
                    .or_else(|| data.get("amount").and_then(|v| v.as_u64()))
                    .ok_or_else(|| NineSError::Other("no 'amount_sat'".into()))?;
                let fee_rate = data.get("fee_rate").and_then(|v| v.as_f64());
                let fee_sat = self.wallet.estimate_fee(to, amt, fee_rate)?;
                Ok(Scroll::new(
                    "/wallet/fee-estimate",
                    json!({"fee_sat": fee_sat, "fee": fee_sat, "to": to, "amount_sat": amt}),
                ))
            }
            _ => Err(NineSError::Other(format!("unknown: {}", path))),
        }
    }

    fn list(&self, _: &str) -> NineSResult<Vec<String>> { Ok(paths::ALL.iter().map(|s| (*s).into()).collect()) }
}

fn uuid() -> String { use std::time::{SystemTime, UNIX_EPOCH}; format!("{:016x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() & 0xFFFFFFFFFFFFFFFF) }

fn format_btc_amount(amount_sat: u64) -> String {
    let whole = amount_sat / 100_000_000;
    let frac = amount_sat % 100_000_000;
    format!("{}.{:08}", whole, frac)
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for &b in value.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(not(feature = "wallet"))]
pub struct WalletNamespace;

#[cfg(not(feature = "wallet"))]
impl Namespace for WalletNamespace {
    fn read(&self, _: &str) -> NineSResult<Option<Scroll>> { Err(NineSError::Other("No wallet".into())) }
    fn write(&self, _: &str, _: Value) -> NineSResult<Scroll> { Err(NineSError::Other("No wallet".into())) }
    fn list(&self, _: &str) -> NineSResult<Vec<String>> { Ok(vec![]) }
}
