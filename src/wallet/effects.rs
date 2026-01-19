//! BitcoinEffectHandler - Async BDK operations for /external/bitcoin/**

use async_trait::async_trait;
use nine_s_core::prelude::*;
use nine_s_store::Store;
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use crate::mind::EffectHandler;
use crate::wallet::bdk::BdkWallet;

pub struct BitcoinEffectHandler {
    wallet: Arc<RwLock<Option<BdkWallet>>>,
    store: Arc<Store>,
}

impl BitcoinEffectHandler {
    pub fn new(wallet: Arc<RwLock<Option<BdkWallet>>>, store: Arc<Store>) -> Self { Self { wallet, store } }

    async fn do_sync(&self) -> anyhow::Result<Value> {
        let (wallet, store) = (self.wallet.clone(), self.store.clone());
        tokio::task::spawn_blocking(move || -> anyhow::Result<Value> {
            let mut guard = wallet.write().map_err(|_| anyhow::anyhow!("lock"))?;
            let w = guard.as_mut().ok_or_else(|| anyhow::anyhow!("no wallet"))?;
            w.sync().map_err(|e| anyhow::anyhow!("{}", e))?;
            let b = w.balance().map_err(|e| anyhow::anyhow!("{}", e))?;
            let txs = w.transactions(50).map_err(|e| anyhow::anyhow!("{}", e))?;
            drop(guard);
            let data = json!({"confirmed": b.confirmed, "pending": b.trusted_pending + b.untrusted_pending, "immature": b.immature, "total": b.confirmed + b.trusted_pending + b.untrusted_pending});
            store.write_scroll(Scroll { key: "/wallet/balance".into(), type_: "wallet/balance@v1".into(), metadata: Metadata::default().with_produced_by("effects"), data: data.clone() }).map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(json!({"synced": true, "balance": data, "tx_count": txs.len()}))
        }).await?
    }

    async fn do_send(&self, scroll: &Scroll) -> anyhow::Result<Value> {
        let to = scroll.data["to"].as_str().ok_or_else(|| anyhow::anyhow!("no 'to'"))?.to_string();
        let amount = scroll.data.get("amount_sat")
            .and_then(|v| v.as_u64())
            .or_else(|| scroll.data.get("amount").and_then(|v| v.as_u64()))
            .ok_or_else(|| anyhow::anyhow!("no 'amount_sat'"))?;
        let fee_rate = scroll.data["fee_rate"].as_f64();
        let wallet = self.wallet.clone();
        let txid = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
            let mut guard = wallet.write().map_err(|_| anyhow::anyhow!("lock"))?;
            guard.as_mut().ok_or_else(|| anyhow::anyhow!("no wallet"))?.send(&to, amount, fee_rate).map_err(|e| anyhow::anyhow!("{}", e))
        }).await??;
        Ok(json!({"success": true, "txid": txid, "to": scroll.data["to"], "amount_sat": amount}))
    }
}

#[async_trait]
impl EffectHandler for BitcoinEffectHandler {
    fn watches(&self) -> &str { "/external/bitcoin" }
    async fn execute(&self, scroll: &Scroll) -> anyhow::Result<Value> {
        if scroll.key.contains("/sync/") { self.do_sync().await }
        else if scroll.key.contains("/send/") { self.do_send(scroll).await }
        else { Err(anyhow::anyhow!("Unknown: {}", scroll.key)) }
    }
}
