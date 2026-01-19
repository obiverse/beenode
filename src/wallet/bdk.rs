//! BDK - Minimal Bitcoin wallet with file-based persistence
//!
//! Thin wrapper over bdk_wallet 2.x with bdk_file_store.
//! Receives 64-byte seed from layer 0. Master mnemonic never crosses boundary.

use nine_s_core::errors::{NineSError, NineSResult};

#[derive(Debug, Clone, Default)]
pub struct WalletBalance {
    pub confirmed: u64,
    pub trusted_pending: u64,
    pub untrusted_pending: u64,
    pub immature: u64,
}

#[derive(Debug, Clone)]
pub struct TransactionDetails {
    pub txid: String,
    pub received: u64,
    pub sent: u64,
    pub fee: Option<u64>,
    pub confirmed: bool,
    pub timestamp: Option<u64>,
    pub block_height: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UtxoDetails {
    pub txid: String,
    pub vout: u32,
    pub amount_sat: u64,
    pub address: Option<String>,
    pub is_change: bool,
}

#[cfg(feature = "wallet")]
mod inner {
    use super::*;
    use bdk_electrum::{electrum_client::Client, BdkElectrumClient};
    use bdk_wallet::{
        bitcoin::{bip32::Xpriv, Address, Network},
        file_store::Store as FileStore,
        template::Bip84,
        ChangeSet, KeychainKind, PersistedWallet, Wallet,
    };
    use std::path::Path;
    use std::str::FromStr;
    use std::sync::Mutex;

    const MAGIC: &[u8] = b"beenode0";

    type PW = PersistedWallet<FileStore<ChangeSet>>;

    /// Sync backend for blockchain data
    enum SyncBackend {
        Electrum(BdkElectrumClient<Client>),
        #[cfg(feature = "bitcoind-rpc")]
        Rpc { url: String, user: String, pass: String },
    }

    pub struct BdkWallet {
        wallet: Mutex<PW>,
        db: Mutex<FileStore<ChangeSet>>,
        backend: SyncBackend,
        network: Network,
    }

    impl BdkWallet {
        /// Create or load wallet from file store with Electrum backend
        pub fn open(seed: &[u8; 64], network: Network, db_path: &Path, electrum_url: Option<&str>) -> NineSResult<Self> {
            let (wallet, db) = Self::create_wallet(seed, network, db_path)?;

            let url = electrum_url.unwrap_or(Self::default_url(network));
            let electrum = Client::new(url)
                .map_err(|e| NineSError::Other(format!("Electrum: {}", e)))?;

            Ok(Self {
                wallet: Mutex::new(wallet),
                db: Mutex::new(db),
                backend: SyncBackend::Electrum(BdkElectrumClient::new(electrum)),
                network,
            })
        }

        /// Create or load wallet from file store with bitcoind RPC backend
        #[cfg(feature = "bitcoind-rpc")]
        pub fn open_rpc(seed: &[u8; 64], network: Network, db_path: &Path, rpc_url: &str, rpc_user: &str, rpc_pass: &str) -> NineSResult<Self> {
            let (wallet, db) = Self::create_wallet(seed, network, db_path)?;

            Ok(Self {
                wallet: Mutex::new(wallet),
                db: Mutex::new(db),
                backend: SyncBackend::Rpc {
                    url: rpc_url.to_string(),
                    user: rpc_user.to_string(),
                    pass: rpc_pass.to_string()
                },
                network,
            })
        }

        fn create_wallet(seed: &[u8; 64], network: Network, db_path: &Path) -> NineSResult<(PW, FileStore<ChangeSet>)> {
            let xprv = Xpriv::new_master(network, seed)
                .map_err(|e| NineSError::Other(format!("Key derivation: {}", e)))?;

            let ext = Bip84(xprv, KeychainKind::External);
            let int = Bip84(xprv, KeychainKind::Internal);

            // Try to load existing wallet with descriptor validation
            let mut db: FileStore<ChangeSet> = FileStore::load_or_create(MAGIC, db_path)
                .map_err(|e| NineSError::Other(format!("FileStore: {}", e)))?.0;

            // Check if stored descriptors match current seed, extract keys for signing
            let wallet_opt = Wallet::load()
                .descriptor(KeychainKind::External, Some(ext.clone()))
                .descriptor(KeychainKind::Internal, Some(int.clone()))
                .extract_keys()
                .load_wallet(&mut db)
                .map_err(|e| NineSError::Other(format!("Load wallet: {}", e)))?;

            let wallet = match wallet_opt {
                Some(w) => w,  // Descriptors match, use existing wallet
                None => {
                    // Descriptors don't match or no wallet exists, create fresh
                    drop(db);
                    let _ = std::fs::remove_file(db_path);
                    let mut db = FileStore::load_or_create(MAGIC, db_path)
                        .map_err(|e| NineSError::Other(format!("FileStore: {}", e)))?.0;
                    let w = Wallet::create(ext, int)
                        .network(network)
                        .create_wallet(&mut db)
                        .map_err(|e| NineSError::Other(format!("Create wallet: {}", e)))?;
                    return Ok((w, db));
                }
            };

            Ok((wallet, db))
        }

        fn default_url(network: Network) -> &'static str {
            match network {
                Network::Bitcoin => "ssl://electrum.blockstream.info:50002",
                Network::Testnet => "ssl://electrum.blockstream.info:60002",
                Network::Signet => "ssl://mempool.space:60602",
                _ => "ssl://electrum.blockstream.info:50002",
            }
        }

        fn persist(&self) -> NineSResult<()> {
            let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
            let mut db = self.db.lock().map_err(|_| NineSError::Other("lock".into()))?;
            wallet.persist(&mut *db).map_err(|e| NineSError::Other(format!("Persist: {}", e)))?;
            Ok(())
        }

        pub fn balance(&self) -> NineSResult<WalletBalance> {
            let wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
            let b = wallet.balance();
            Ok(WalletBalance {
                confirmed: b.confirmed.to_sat(),
                trusted_pending: b.trusted_pending.to_sat(),
                untrusted_pending: b.untrusted_pending.to_sat(),
                immature: b.immature.to_sat(),
            })
        }

        pub fn receive_address(&self) -> NineSResult<String> {
            let addr = {
                let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
                wallet.next_unused_address(KeychainKind::External).address.to_string()
            };
            self.persist()?;
            Ok(addr)
        }

        pub fn new_address(&self) -> NineSResult<String> {
            let addr = {
                let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
                wallet.reveal_next_address(KeychainKind::External).address.to_string()
            };
            self.persist()?;
            Ok(addr)
        }

        pub fn sync(&self) -> NineSResult<()> {
            match &self.backend {
                SyncBackend::Electrum(client) => self.sync_electrum(client),
                #[cfg(feature = "bitcoind-rpc")]
                SyncBackend::Rpc { url, user, pass } => self.sync_rpc(url, user, pass),
            }
        }

        fn sync_electrum(&self, client: &BdkElectrumClient<Client>) -> NineSResult<()> {
            {
                let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
                let request = wallet.start_full_scan();
                let update = client.full_scan(request, 10, 10, false)
                    .map_err(|e| NineSError::Other(format!("Sync: {}", e)))?;
                wallet.apply_update(update).map_err(|e| NineSError::Other(format!("Apply: {}", e)))?;
            }
            self.persist()?;
            Ok(())
        }

        #[cfg(feature = "bitcoind-rpc")]
        fn sync_rpc(&self, url: &str, user: &str, pass: &str) -> NineSResult<()> {
            use bdk_bitcoind_rpc::Emitter;
            use bitcoincore_rpc::{Auth, Client as RpcClient, RpcApi};
            use bdk_wallet::chain::{BlockId, local_chain::CheckPoint};

            let rpc = RpcClient::new(url, Auth::UserPass(user.to_string(), pass.to_string()))
                .map_err(|e| NineSError::Other(format!("RPC connect: {}", e)))?;

            // Get server's genesis to use as starting checkpoint
            let genesis_hash = rpc.get_block_hash(0)
                .map_err(|e| NineSError::Other(format!("RPC genesis: {}", e)))?;
            let genesis_cp = CheckPoint::new(BlockId { height: 0, hash: genesis_hash });

            {
                let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
                let mut emitter = Emitter::new(&rpc, genesis_cp, 0, std::iter::empty::<std::sync::Arc<bdk_wallet::bitcoin::Transaction>>());

                // Fetch blocks until tip
                loop {
                    match emitter.next_block() {
                        Ok(Some(block_event)) => {
                            let height = block_event.block_height();
                            let connected_to = block_event.connected_to();
                            wallet.apply_block_connected_to(&block_event.block, height, connected_to)
                                .map_err(|e| NineSError::Other(format!("Apply block: {}", e)))?;
                        }
                        Ok(None) => break, // Reached tip
                        Err(e) => return Err(NineSError::Other(format!("RPC block: {}", e))),
                    }
                }

                // Sync mempool
                if let Ok(mempool) = emitter.mempool() {
                    wallet.apply_unconfirmed_txs(mempool.update.iter().map(|(tx, time)| ((**tx).clone(), *time)));
                }
            }
            self.persist()?;
            Ok(())
        }

        pub fn transactions(&self, limit: usize) -> NineSResult<Vec<TransactionDetails>> {
            let wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
            Ok(wallet.transactions().take(limit).map(|tx| {
                let (confirmed, timestamp, block_height) = match tx.chain_position {
                    bdk_wallet::chain::ChainPosition::Confirmed { anchor, .. } =>
                        (true, Some(anchor.confirmation_time as u64), Some(anchor.block_id.height)),
                    bdk_wallet::chain::ChainPosition::Unconfirmed { .. } => (false, None, None),
                };
                let (sent, received) = wallet.sent_and_received(&tx.tx_node.tx);
                TransactionDetails {
                    txid: tx.tx_node.txid.to_string(),
                    received: received.to_sat(),
                    sent: sent.to_sat(),
                    fee: wallet.calculate_fee(&tx.tx_node.tx).ok().map(|f| f.to_sat()),
                    confirmed, timestamp, block_height,
                }
            }).collect())
        }

        pub fn send(&self, to: &str, amount_sat: u64, fee_rate: Option<f64>) -> NineSResult<String> {
            use bdk_wallet::bitcoin::Amount;

            let address = Address::from_str(to)
                .map_err(|e| NineSError::Other(format!("Address: {}", e)))?
                .require_network(self.network)
                .map_err(|e| NineSError::Other(format!("Network: {}", e)))?;

            let tx = {
                let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
                let mut builder = wallet.build_tx();
                builder.add_recipient(address.script_pubkey(), Amount::from_sat(amount_sat));
                if let Some(rate) = fee_rate {
                    builder.fee_rate(bdk_wallet::bitcoin::FeeRate::from_sat_per_vb(rate as u64).unwrap());
                }

                let mut psbt = builder.finish().map_err(|e| NineSError::Other(format!("Build: {}", e)))?;
                #[allow(deprecated)]
                wallet.sign(&mut psbt, bdk_wallet::SignOptions::default())
                    .map_err(|e| NineSError::Other(format!("Sign: {}", e)))?;

                psbt.extract_tx().map_err(|e| NineSError::Other(format!("Extract: {}", e)))?
            };

            let txid = tx.compute_txid();

            // Broadcast based on backend
            match &self.backend {
                SyncBackend::Electrum(client) => {
                    use bdk_electrum::electrum_client::ElectrumApi;
                    client.inner.transaction_broadcast(&tx)
                        .map_err(|e| NineSError::Other(format!("Broadcast: {}", e)))?;
                }
                #[cfg(feature = "bitcoind-rpc")]
                SyncBackend::Rpc { url, user, pass } => {
                    use bitcoincore_rpc::{Auth, Client as RpcClient, RpcApi};
                    let rpc = RpcClient::new(url, Auth::UserPass(user.clone(), pass.clone()))
                        .map_err(|e| NineSError::Other(format!("RPC connect: {}", e)))?;
                    rpc.send_raw_transaction(&tx)
                        .map_err(|e| NineSError::Other(format!("RPC broadcast: {}", e)))?;
                }
            }

            self.persist()?;
            Ok(txid.to_string())
        }

        pub fn estimate_fee(&self, to: &str, amount_sat: u64, fee_rate: Option<f64>) -> NineSResult<u64> {
            use bdk_wallet::bitcoin::Amount;

            let address = Address::from_str(to)
                .map_err(|e| NineSError::Other(format!("Address: {}", e)))?
                .require_network(self.network)
                .map_err(|e| NineSError::Other(format!("Network: {}", e)))?;

            let mut wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
            let mut builder = wallet.build_tx();
            builder.add_recipient(address.script_pubkey(), Amount::from_sat(amount_sat));
            if let Some(rate) = fee_rate {
                builder.fee_rate(bdk_wallet::bitcoin::FeeRate::from_sat_per_vb(rate as u64).unwrap());
            }
            let psbt = builder.finish().map_err(|e| NineSError::Other(format!("Fee: {}", e)))?;
            Ok(psbt.fee().map_err(|e| NineSError::Other(format!("Calc: {}", e)))?.to_sat())
        }

        pub fn list_unspent(&self) -> NineSResult<Vec<UtxoDetails>> {
            let wallet = self.wallet.lock().map_err(|_| NineSError::Other("lock".into()))?;
            Ok(wallet.list_unspent().map(|utxo| {
                let address = Address::from_script(&utxo.txout.script_pubkey, self.network)
                    .ok()
                    .map(|a| a.to_string());
                UtxoDetails {
                    txid: utxo.outpoint.txid.to_string(),
                    vout: utxo.outpoint.vout,
                    amount_sat: utxo.txout.value.to_sat(),
                    address,
                    is_change: utxo.keychain == KeychainKind::Internal,
                }
            }).collect())
        }
    }
}

#[cfg(feature = "wallet")]
pub use inner::BdkWallet;

#[cfg(not(feature = "wallet"))]
pub struct BdkWallet;

#[cfg(not(feature = "wallet"))]
impl BdkWallet {
    pub fn balance(&self) -> NineSResult<WalletBalance> { Ok(WalletBalance::default()) }
    pub fn receive_address(&self) -> NineSResult<String> { Err(NineSError::Other("No wallet".into())) }
    pub fn new_address(&self) -> NineSResult<String> { Err(NineSError::Other("No wallet".into())) }
    pub fn sync(&self) -> NineSResult<()> { Err(NineSError::Other("No wallet".into())) }
    pub fn transactions(&self, _: usize) -> NineSResult<Vec<TransactionDetails>> { Ok(vec![]) }
    pub fn send(&self, _: &str, _: u64, _: Option<f64>) -> NineSResult<String> { Err(NineSError::Other("No wallet".into())) }
    pub fn estimate_fee(&self, _: &str, _: u64, _: Option<f64>) -> NineSResult<u64> { Err(NineSError::Other("No wallet".into())) }
    pub fn list_unspent(&self) -> NineSResult<Vec<UtxoDetails>> { Ok(vec![]) }
}
