use crate::utils::Result;
use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use kv::{Bucket, Config, Raw, Store};

pub const DB_PATH: &str = "./state";

pub const CHAIN_BUCKET: &str = "blockchain";
pub const WALLETS_BUCKET: &str = "wallets";

pub struct AppStore(pub Store);

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

    pub fn get_blocks_bucket(&self) -> Result<Bucket<'a, Raw, Raw>> {
        let store = &self.0;

        let bucket = store.bucket::<Raw, Raw>(Some(CHAIN_BUCKET)).unwrap();

        Ok(bucket)
    }

    pub fn get_wallets_bucket(&self) -> Result<Bucket<'a, Vec<u8>, Vec<u8>>> {
        let store = &self.0;

        let bucket = store
            .bucket::<Vec<u8>, Vec<u8>>(Some(WALLETS_BUCKET))
            .unwrap();

        Ok(bucket)
    }
}
