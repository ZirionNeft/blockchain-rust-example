use crate::utils::Result;
use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use kv::{Bucket, Config, Raw, Store};

pub const DB_PATH: &str = "./state";

pub const CHAIN_BUCKET: &str = "blockchain";
pub const WALLETS_BUCKET: &str = "wallets";

lazy_static! {
    pub static ref STORE: Arc<Mutex<Store>> = Arc::new(Mutex::new(AppStore::init_store().unwrap()));
}

pub struct AppStore;

impl<'a> AppStore {
    pub fn get_store() -> Arc<Mutex<Store>> {
        Arc::clone(&STORE)
    }

    pub fn get_blocks_bucket() -> Result<Bucket<'a, Raw, Raw>> {
        let store = Self::get_store();
        let store = store.lock().unwrap();

        let bucket = store
            .bucket::<Raw, Raw>(Some(CHAIN_BUCKET))
            .expect("Blocks bucket init error");

        Ok(bucket)
    }

    pub fn get_wallets_bucket() -> Result<Bucket<'a, Vec<u8>, Vec<u8>>> {
        let store = Self::get_store();
        let store = store.lock().unwrap();

        let bucket = store
            .bucket::<Vec<u8>, Vec<u8>>(Some(WALLETS_BUCKET))
            .expect("Wallets bucket init error");

        Ok(bucket)
    }

    fn init_store() -> Result<Store> {
        let cfg: Config;
        if File::open(DB_PATH).is_ok() {
            cfg = Config::load(DB_PATH)?;
        } else {
            cfg = Config::new(DB_PATH);
        }

        Store::new(cfg).map_err(|e| e.into())
    }
}
