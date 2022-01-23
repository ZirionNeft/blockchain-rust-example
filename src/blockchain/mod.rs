use std::{collections::HashMap, error, fs::File};

use kv::{Bucket, Config, Raw, Store};

use crate::utils::HashHex;

use self::{block::Block, proof_of_work::ProofOfWork, transaction::Transaction};

pub(crate) mod block;
pub(crate) mod proof_of_work;
pub(crate) mod transaction;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Clone)]
pub struct Blockchain<'a> {
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

fn init_store() -> Result<Store> {
    let cfg: Config;
    if File::open(DB_PATH).is_ok() {
        cfg = Config::load(DB_PATH)?;
    } else {
        cfg = Config::new(DB_PATH);
    }

    Store::new(cfg).map_err(|e| e.into())
}

type Accumulated = u32;

impl<'a> Blockchain<'a> {
    pub fn new(address: Option<String>) -> Result<Blockchain<'a>> {
        let store = init_store()?;

        let bucket_name = &BUCKET_NAME.to_string();

        let tip_hash: HashHex;
        if store.buckets().contains(bucket_name) {
            let bucket = store.bucket::<Raw, Raw>(Some(BUCKET_NAME))?;

            tip_hash = bucket
                .get(b"1")?
                .expect("Hash value is None")
                .to_vec()
                .into();
        } else {
            let bucket = store.bucket::<Raw, Raw>(Some(BUCKET_NAME))?;

            let genesis_block = Block::new_genesis(
                address.ok_or("Blockchain is not initialized yet and address is undefined")?,
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

    pub fn exists() -> bool {
        let store = init_store().expect("Store init error");

        if store.buckets().contains(&BUCKET_NAME.to_string()) {
            let bucket = store
                .bucket::<Raw, Raw>(Some(BUCKET_NAME))
                .expect("Can't get bucket");

            if let Ok(Some(_tip)) = bucket.get(b"1") {
                return true;
            }
        }

        false
    }

    pub fn add_block(
        &mut self,
        transactions: Vec<Transaction>,
    ) -> std::result::Result<Block, kv::Error> {
        let bucket = self.store.bucket::<Raw, Raw>(Some(BUCKET_NAME))?;

        let last_hash: HashHex = bucket
            .get(b"1")?
            .expect("Tip block hash value is None")
            .to_vec()
            .into();

        let new_block: Block = Block::new(last_hash, transactions);

        bucket.transaction(|txn| {
            let raw_block: Raw = new_block.clone().into();

            txn.set(new_block.hash.0.as_slice(), raw_block)?;
            txn.set(b"1", new_block.hash.0.as_slice())?;

            Ok(())
        })?;

        self.tip = new_block.hash.to_owned();

        Ok(new_block)
    }

    pub fn find_unspent_transactions(&self, address: &str) -> Vec<Transaction> {
        let mut unspent_transactions = vec![];
        let mut spent_tx_outputs = HashMap::<HashHex, Vec<i32>>::new();

        let mut iterator = self.to_owned();

        let address = &address.to_string();

        loop {
            let block = iterator
                .next()
                .expect("Can't read the block from blockchain during finding unspent transactions");

            for tx in block.transactions.iter() {
                let tx_id = tx.id.to_owned();

                'outputs: for (output_index, output) in tx.outputs.iter().enumerate() {
                    if let Some(spent_output_indexes) = &spent_tx_outputs.get(&tx_id) {
                        for spent_output_index in spent_output_indexes.iter() {
                            if *spent_output_index == output_index as i32 {
                                continue 'outputs;
                            }
                        }
                    }

                    if output.is_unlockable_with(address) {
                        unspent_transactions.push(tx.clone());
                    }
                }

                if !tx.is_coinbase() {
                    for input in tx.inputs.iter() {
                        if input.can_unlock_output_with(address) {
                            let input_tx_id = input.tx_id.to_owned();

                            spent_tx_outputs
                                .entry(input_tx_id)
                                .or_insert_with(Vec::new)
                                .push(input.output_index);
                        }
                    }
                }
            }

            if block.prev_hash.0.is_empty() {
                break;
            }
        }

        unspent_transactions
    }

    pub fn find_spendable_outputs(
        &self,
        address: &str,
        amount: u32,
    ) -> (Accumulated, HashMap<HashHex, Vec<i32>>) {
        let mut unspent_outputs = HashMap::<HashHex, Vec<i32>>::new();

        let mut accumulated = 0;

        let address = &address.to_string();

        let unspent_transactions = self.find_unspent_transactions(address);

        'outer: for tx in unspent_transactions {
            let tx_id = tx.id;

            for (output_index, output) in tx.outputs.iter().enumerate() {
                if output.is_unlockable_with(address) && accumulated < amount {
                    accumulated += output.value;

                    unspent_outputs
                        .entry(tx_id.to_owned())
                        .or_insert_with(Vec::new)
                        .push(output_index as i32);

                    if accumulated >= amount {
                        break 'outer;
                    }
                }
            }
        }

        (accumulated, unspent_outputs)
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
                    "[!] Bucket loaded with size: {:?}",
                    state.bucket.as_ref()?.clone().len() - 1
                );

                state.bucket.as_ref()?
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
