use crate::{blockchain::transaction::TXOutput, utils::Result};
use std::{
    fs::File,
    sync::{Arc, Mutex}, collections::HashMap,
};

use kv::{Bucket, Config, Json, Raw, Store};

pub const DB_PATH: &str = "./store";

pub const BLOCKS_BUCKET: &str = "blocks";
pub const CHAINSTATE_BUCKET: &str = "chainstate";
pub const WALLETS_BUCKET: &str = "wallets";

pub struct AppStore(pub Store);

type TxId = Vec<u8>;
type WalletAddress = Vec<u8>;
type SecretKey = Vec<u8>;
type BlockHash = Vec<u8>;
type BlockJson = Raw;

impl<'a> AppStore {
    pub fn new() -> Arc<Mutex<Self>> {
        let cfg: Config;
        if File::open(DB_PATH).is_ok() {
            cfg = Config::load(DB_PATH).unwrap();
        } else {
            cfg = Config::new(DB_PATH);
        }

        let store = Store::new(cfg).unwrap();

        Arc::new(Mutex::new(AppStore(store)))
    }

    pub fn get_blocks_bucket(&self) -> Result<Bucket<'a, Vec<u8>, Raw>> {
        let store = &self.0;

        let bucket = store.bucket::<BlockHash, BlockJson>(Some(BLOCKS_BUCKET)).unwrap();

        Ok(bucket)
    }

    pub fn get_chainstate_bucket(&self) -> Result<Bucket<'a, Vec<u8>, Json<HashMap<i32, TXOutput>>>> {
        let store = &self.0;

        let bucket = store
            .bucket::<TxId, Json<HashMap<i32, TXOutput>>>(Some(CHAINSTATE_BUCKET))
            .unwrap();

        Ok(bucket)
    }

    pub fn get_wallets_bucket(&self) -> Result<Bucket<'a, Vec<u8>, Vec<u8>>> {
        let store = &self.0;

        let bucket = store
            .bucket::<WalletAddress, SecretKey>(Some(WALLETS_BUCKET))
            .unwrap();

        Ok(bucket)
    }
}
