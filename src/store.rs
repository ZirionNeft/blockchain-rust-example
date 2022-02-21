use crate::utils::Result;
use std::fs::File;

use kv::{Config, Store};

pub const DB_PATH: &str = "./state";

pub const CHAIN_BUCKET: &str = "blockchain";
pub const WALLETS_BUCKET: &str = "wallets";

pub fn init_store() -> Result<Store> {
    let cfg: Config;
    if File::open(DB_PATH).is_ok() {
        cfg = Config::load(DB_PATH)?;
    } else {
        cfg = Config::new(DB_PATH);
    }

    Store::new(cfg).map_err(|e| e.into())
}
