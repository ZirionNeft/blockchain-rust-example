use std::collections::HashMap;

use kv::{Batch, Json};

use crate::utils::{HashHex, Result};

use super::{block::Block, transaction::TXOutput, Blockchain};

pub struct UTXOSet<'a> {
    pub blockchain: &'a Blockchain<'a>,
}

pub type Accumulated = u32;

impl<'a> UTXOSet<'a> {
    // TODO: Неправильно работает апдейт - у коинбейз тразакций одинаковый айди, поэтому данные перезаписываются
    pub fn update(&self, block: &Block) -> Result<()> {
        let bucket = self.blockchain.store.get_chainstate_bucket()?;

        bucket.transaction(|tx| {
            for bc_tx in block.transactions.iter() {
                if !bc_tx.is_coinbase() {
                    for input in bc_tx.inputs.iter() {
                        let tx_id = input.tx_id.to_owned().to_vec();

                        let stored_outputs = tx.get(tx_id.clone())?;
                        let stored_outputs = match stored_outputs {
                            Some(v) => v.0,
                            None => continue,
                        };

                        let filtered: HashMap<i32, TXOutput> = stored_outputs
                            .iter()
                            .filter_map(|(index, out)| {
                                if *index != input.output_index {
                                    return Some((*index, out.clone()));
                                }
                                None
                            })
                            .collect();

                        if filtered.is_empty() {
                            tx.remove(tx_id)?;
                        } else {
                            tx.set(tx_id, Json(filtered))?;
                        }
                    }
                }

                let tx_id = bc_tx.id.to_owned().to_vec();
                let outputs: HashMap<i32, TXOutput> = bc_tx
                    .outputs
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, out)| (index as i32, out))
                    .collect();
                let outputs = Json(outputs);

                tx.set(tx_id, outputs)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn reindex(&self) -> Result<()> {
        println!("-> Chainstate reindex begining...");

        let bucket = self.blockchain.store.get_chainstate_bucket()?;

        bucket.clear()?;
        println!("[!] Chainstate store cleared");

        let bc = self.blockchain.clone();
        let mut batch = Batch::new();

        for block in bc {
            for bc_tx in block.transactions.iter() {
                let tx_id = bc_tx.id.clone();
                let outputs: HashMap<i32, TXOutput> = bc_tx
                    .outputs
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, out)| (index as i32, out))
                    .collect();
                let outputs = Json(outputs);

                batch.set(tx_id.to_vec(), outputs)?;
            }
        }

        bucket.batch(batch)?;

        println!("-> Chainstate reindex completed!");

        Ok(())
    }

    pub fn find_utxo(&self, pub_key_hash: &HashHex) -> Result<Vec<TXOutput>> {
        let bucket = self.blockchain.store.get_chainstate_bucket()?;

        let outputs = bucket
            .iter()
            .filter_map(|tx_item| {
                let tx_item = tx_item.unwrap();
                let outputs: Json<HashMap<i32, TXOutput>> = tx_item.value().unwrap();

                let tx_filtered_outputs = outputs
                    .0
                    .values()
                    .filter(|output| output.is_locked_with(pub_key_hash))
                    .cloned()
                    .collect::<Vec<TXOutput>>();

                if !tx_filtered_outputs.is_empty() {
                    return Some(tx_filtered_outputs);
                }

                None
            })
            .flatten()
            .collect::<Vec<TXOutput>>();

        Ok(outputs)
    }

    pub fn find_spendable_outputs(
        &self,
        pub_key_hash: &HashHex,
        amount: u32,
    ) -> Result<(Accumulated, HashMap<HashHex, Vec<i32>>)> {
        let bucket = self.blockchain.store.get_chainstate_bucket()?;

        let mut unspent_outputs = HashMap::<HashHex, Vec<i32>>::new();
        let mut accumulated = 0;

        for item in bucket.iter() {
            let item = item?;
            let tx_id: Vec<u8> = item.key()?;
            let outputs = item.value::<Json<HashMap<i32, TXOutput>>>()?.0;

            for (output_index, output) in outputs {
                if output.is_locked_with(pub_key_hash) && accumulated < amount {
                    accumulated += output.value;

                    unspent_outputs
                        .entry(tx_id.clone().into())
                        .or_insert_with(Vec::new)
                        .push(output_index as i32);

                    if accumulated >= amount {
                        break;
                    }
                }
            }
        }

        Ok((accumulated, unspent_outputs))
    }
}
