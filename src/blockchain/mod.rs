use std::fs::File;

use kv::{Bucket, Config, Error, Raw, Store};
use serde_json::{json, Value as JsonValue};

use crate::utils::{self, get_current_time};

use self::{
    block::{Block, HashHex},
    proof_of_work::ProofOfWork,
};

pub(crate) mod block;
pub(crate) mod proof_of_work;

#[derive(Clone)]
pub(crate) struct Blockchain<'a> {
    pub tip: HashHex,
    pub store: Store,
    iterator_state: IteratorState<'a>,
}

#[derive(Clone)]
struct IteratorState<'a> {
    current_hash: Option<HashHex>,
    bucket: Option<Bucket<'a, Raw, Raw>>,
}

const DB_PATH: &str = "./chainstate";
const BUCKET_NAME: &str = "chainstate";

impl<'a> Blockchain<'a> {
    pub fn new() -> Result<Blockchain<'a>, Error> {
        let cfg: Config;
        if File::open(DB_PATH).is_ok() {
            cfg = Config::load(DB_PATH)?;
        } else {
            cfg = Config::new(DB_PATH);
        }

        let store = Store::new(cfg)?;

        let tip_hash: HashHex;
        if store.buckets().contains(&BUCKET_NAME.to_string()) {
            let bucket = store.bucket::<Raw, Raw>(Some(BUCKET_NAME))?;

            tip_hash = bucket
                .get(b"1")?
                .expect("Hash value is None")
                .to_vec()
                .into();
        } else {
            let bucket = store.bucket::<Raw, Raw>(Some(BUCKET_NAME))?;

            let genesis_block = Block::new(
                HashHex(vec![]),
                json!({
                    "genesis": "Let's get it started!"
                }),
                get_current_time(),
            );

            bucket.transaction(|txn| {
                let raw_block: Raw = genesis_block.clone().into();

                txn.set(genesis_block.hash.0.as_slice(), raw_block)?;
                txn.set(b"1", genesis_block.hash.0.as_slice())?;

                Ok(())
            })?;

            tip_hash = genesis_block.hash;
        }

        Ok(Blockchain {
            iterator_state: IteratorState {
                bucket: None,
                current_hash: None,
            },
            tip: tip_hash,
            store,
        })
    }

    pub fn add_block(&mut self, payload: JsonValue) -> Result<Block, kv::Error> {
        let bucket = self.store.bucket::<Raw, Raw>(Some(BUCKET_NAME))?;

        let last_hash: HashHex = bucket
            .get(b"1")?
            .expect("Tip block hash value is None")
            .to_vec()
            .into();

        let new_block: Block = Block::new(last_hash, payload, utils::get_current_time());

        bucket.transaction(|txn| {
            let raw_block: Raw = new_block.clone().into();

            txn.set(new_block.hash.0.as_slice(), raw_block)?;
            txn.set(b"1", new_block.hash.0.as_slice())?;

            Ok(())
        })?;

        self.tip = new_block.hash.clone();

        Ok(new_block)
    }
}

impl<'a> Iterator for Blockchain<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let state = &mut self.iterator_state;

        let bucket = match &state.bucket {
            Some(s) => s,
            None => {
                let bucket = self
                    .store
                    .bucket::<Raw, Raw>(Some(BUCKET_NAME))
                    .expect("Bucket retrieveing during iterating error");

                state.bucket = Some(bucket);

                println!(
                    "[!] Bucket size: {:?}",
                    state.bucket.as_ref()?.clone().len() - 1
                );

                state.bucket.as_ref().unwrap()
            }
        };

        let current_hash = match &state.current_hash {
            Some(v) => v,
            None => &self.tip,
        };

        if current_hash.0.is_empty() {
            state.current_hash = None;
            state.bucket = None;

            return None;
        }

        let raw_block = bucket
            .get(current_hash.0.as_slice())
            .expect("Block getting during iterating error")
            .unwrap_or_else(|| {
                panic!(
                    "Block '{}' is None",
                    serde_json::to_string(current_hash).unwrap()
                )
            });

        let block: Block = raw_block.into();

        let proof_of_work = ProofOfWork::new(&block);

        if !proof_of_work.validate() {
            panic!("Block proof-of-work validation error");
        }

        state.current_hash = Some(block.prev_hash.clone());

        Some(block)
    }
}
