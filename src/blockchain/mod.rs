use crate::{
    store::AppStore,
    store::BLOCKS_BUCKET,
    utils::{HashHex, Result},
};
use kv::{Bucket, Raw};
use p256::ecdsa::SigningKey;
use std::{collections::HashMap, error, fmt};

use self::{block::Block, proof_of_work::ProofOfWork, transaction::Transaction};

pub(crate) mod block;
pub(crate) mod proof_of_work;
pub(crate) mod transaction;
pub(crate) mod utxo_set;
pub(crate) mod wallet;
pub(crate) mod merkle_tree;

#[derive(Debug, Clone)]
struct BadTransactionError;

impl fmt::Display for BadTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Transaction verifying error")
    }
}

impl error::Error for BadTransactionError {}

#[derive(Clone)]
pub struct Blockchain<'a> {
    pub tip: HashHex,
    iterator_state: IteratorState<'a>,
    store: &'a AppStore,
}

#[derive(Clone)]
struct IteratorState<'a> {
    current_hash: Option<HashHex>,
    bucket: Option<Bucket<'a, Vec<u8>, Raw>>,
}

impl<'a> Blockchain<'a> {
    pub fn new(address: Option<String>, store: &'a AppStore) -> Result<Blockchain<'a>> {
        let bucket_name = &BLOCKS_BUCKET.to_string();

        let tip_hash: HashHex;
        if store.0.buckets().contains(bucket_name) {
            let bucket = store.0.bucket::<Raw, Raw>(Some(BLOCKS_BUCKET))?;

            tip_hash = bucket
                .get(b"1")?
                .expect("Tip hash is not found. Try to remove store and re-init blockchain")
                .to_vec()
                .into();
        } else {
            let init_chain = || -> Result<HashHex> {
                let blocks_bucket = store.0.bucket::<Raw, Raw>(Some(BLOCKS_BUCKET))?;

                let genesis_block = Block::new_genesis(
                    address.ok_or("Blockchain is not initialized yet and address is undefined")?,
                    store,
                )?;

                blocks_bucket.transaction(|txn| {
                    let raw_block: Raw = genesis_block.clone().into();

                    txn.set(genesis_block.hash.0.as_slice(), raw_block)?;
                    txn.set(b"1", genesis_block.hash.0.as_slice())?;

                    Ok(())
                })?;

                Ok(genesis_block.hash)
            };

            tip_hash = init_chain().map_err(|e| {
                store
                    .0
                    .drop_bucket(bucket_name)
                    .or_else::<Box<dyn error::Error>, _>(|e| {
                        println!("{:?}", e);
                        println!("(*) Bucket did not deleted");
                        Ok(())
                    })
                    .ok();

                e
            })?;
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

    pub fn exists(store: &AppStore) -> bool {
        let store = &store.0;

        if store.buckets().contains(&BLOCKS_BUCKET.to_string()) {
            let bucket = store.bucket::<Raw, Raw>(Some(BLOCKS_BUCKET)).unwrap();

            if let Ok(Some(_tip)) = bucket.get(b"1") {
                return true;
            }
        }

        false
    }

    pub fn add_block(&mut self, mut transactions: Vec<Transaction>) -> Result<Block> {
        let bucket = self.store.get_blocks_bucket()?;

        let last_hash: HashHex = bucket
            .get(b"1".to_vec())?
            .expect("Tip block hash value is None")
            .to_vec()
            .into();

        for tx in transactions.iter_mut() {
            if !self.verify_transaction(tx) {
                println!("[!] Transactions verification is not passed");
                return Err(Box::new(BadTransactionError));
            }
        }

        let new_block: Block = Block::new(last_hash, transactions);

        bucket.transaction(|txn| {
            let raw_block: Raw = new_block.clone().into();

            txn.set(new_block.hash.0.clone(), raw_block)?;
            txn.set(b"1".to_vec(), new_block.hash.0.as_slice())?;

            Ok(())
        })?;

        self.tip = new_block.hash.to_owned();

        Ok(new_block)
    }

    pub fn find_transaction(&self, id: &HashHex) -> Option<Transaction> {
        let mut iterator = self.to_owned();

        loop {
            let block = iterator.next().unwrap();

            let tx = block
                .transactions
                .iter()
                .find(|&tx| tx.id.0.cmp(&id.0).is_eq());

            match tx {
                Some(v) => return Some(v.clone()),
                None => {
                    if block.prev_hash.0.is_empty() {
                        break;
                    }
                }
            }
        }

        None
    }

    pub fn sign_transaction(&self, tx: &mut Transaction, private_key: &SigningKey) {
        let mut prev_txs: HashMap<HashHex, Transaction> = tx
            .inputs
            .iter()
            .map(|input| {
                let prev_tx = self.find_transaction(&input.tx_id).unwrap();

                (prev_tx.id.clone(), prev_tx)
            })
            .collect();

        tx.sign(&mut prev_txs, private_key);
    }

    pub fn verify_transaction(&self, tx: &mut Transaction) -> bool {
        if tx.is_coinbase() {
            return true;
        }

        let prev_txs: HashMap<HashHex, Transaction> = tx
            .inputs
            .iter()
            .map(|input| {
                let prev_tx = self.find_transaction(&input.tx_id).unwrap();

                (prev_tx.id.clone(), prev_tx)
            })
            .collect();

        tx.verify(&prev_txs)
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
                    .get_blocks_bucket()
                    .expect("Bucket retrieveing during iterating error");

                state.bucket = Some(bucket);

                println!(
                    "[!] Iterator: Blocks in store - {:?}",
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
