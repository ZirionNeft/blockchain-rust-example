use crate::{
    store::AppStore,
    store::CHAIN_BUCKET,
    utils::{HashHex, Result},
};
use kv::{Bucket, Raw};
use p256::ecdsa::SigningKey;
use std::{collections::HashMap, error, fmt};

use self::{
    block::Block,
    proof_of_work::ProofOfWork,
    transaction::{TXOutput, Transaction},
};

pub(crate) mod block;
pub(crate) mod proof_of_work;
pub(crate) mod transaction;
pub(crate) mod wallet;

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
    bucket: Option<Bucket<'a, Raw, Raw>>,
}

type Accumulated = u32;

impl<'a> Blockchain<'a> {
    pub fn new(address: Option<String>, store: &'a AppStore) -> Result<Blockchain<'a>> {
        let bucket_name = &CHAIN_BUCKET.to_string();

        let tip_hash: HashHex;
        if store.0.buckets().contains(bucket_name) {
            let bucket = store.0.bucket::<Raw, Raw>(Some(CHAIN_BUCKET))?;

            tip_hash = bucket
                .get(b"1")?
                .expect("Hash value is None")
                .to_vec()
                .into();
        } else {
            let init_chain = || -> Result<HashHex> {
                let blocks_bucket = store.0.bucket::<Raw, Raw>(Some(CHAIN_BUCKET))?;

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

        println!("-> Blockchain inited!");

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

        if store.buckets().contains(&CHAIN_BUCKET.to_string()) {
            let bucket = store.bucket::<Raw, Raw>(Some(CHAIN_BUCKET)).unwrap();

            if let Ok(Some(_tip)) = bucket.get(b"1") {
                return true;
            }
        }

        false
    }

    pub fn add_block(&mut self, mut transactions: Vec<Transaction>) -> Result<Block> {
        let bucket = self.store.get_blocks_bucket()?;

        let last_hash: HashHex = bucket
            .get(b"1")?
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

            txn.set(new_block.hash.0.as_slice(), raw_block)?;
            txn.set(b"1", new_block.hash.0.as_slice())?;

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

    pub fn find_unspent_transactions(&self, pub_key_hash: &HashHex) -> Vec<Transaction> {
        let mut unspent_transactions = vec![];
        let mut spent_tx_outputs = HashMap::<HashHex, Vec<i32>>::new();

        let mut iterator = self.to_owned();

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

                    if output.is_locked_with(pub_key_hash) {
                        unspent_transactions.push(tx.clone());
                    }
                }

                if !tx.is_coinbase() {
                    for input in tx.inputs.iter() {
                        if input.uses_key(pub_key_hash) {
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

    pub fn find_utxo(&self, pub_key_hash: &HashHex) -> Vec<TXOutput> {
        self.find_unspent_transactions(pub_key_hash)
            .iter()
            .flat_map(|tx| {
                tx.outputs
                    .iter()
                    .filter(|out| out.is_locked_with(pub_key_hash))
                    .cloned()
            })
            .collect()
    }

    pub fn find_spendable_outputs(
        &self,
        pub_key_hash: &HashHex,
        amount: u32,
    ) -> (Accumulated, HashMap<HashHex, Vec<i32>>) {
        let mut unspent_outputs = HashMap::<HashHex, Vec<i32>>::new();

        let mut accumulated = 0;

        let unspent_transactions = self.find_unspent_transactions(pub_key_hash);

        'outer: for tx in unspent_transactions {
            let tx_id = tx.id;

            for (output_index, output) in tx.outputs.iter().enumerate() {
                if output.is_locked_with(pub_key_hash) && accumulated < amount {
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
                    .get_blocks_bucket()
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
