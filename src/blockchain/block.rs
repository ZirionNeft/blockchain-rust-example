use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sha2::{Digest, Sha256};

use crate::utils::{get_current_time, HashHex};

use super::{proof_of_work::ProofOfWork, transaction::Transaction};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub timestamp: String,
    pub transactions: Vec<Transaction>,
    pub hash: HashHex,
    pub prev_hash: HashHex,
    pub nonce: u64,
}

impl Block {
    pub fn new(prev_hash: HashHex, transactions: Vec<Transaction>) -> Self {
        let mut new_block: Block = Block {
            prev_hash,
            transactions,
            timestamp: get_current_time(),
            hash: HashHex(vec![]),
            nonce: 0,
        };

        let proof_of_work = ProofOfWork::new(&new_block);

        let (nonce, hash) = match proof_of_work.run() {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };

        new_block.hash = hash;
        new_block.nonce = nonce;

        new_block
    }

    pub fn new_genesis(address: String) -> Self {
        let tx = Transaction::new_coinbase(address, None).expect("New coinbase transaction error");

        Block::new(HashHex(vec![]), vec![tx])
    }

    pub fn hash_transactions(&self) -> Vec<u8> {
        let transactions = &self.transactions;

        let tx_hashes: Vec<u8> = transactions
            .iter()
            .map(|tx| tx.id.0.to_owned())
            .flatten()
            .collect();

        let mut hasher = Sha256::new();

        hasher.update(tx_hashes);

        hasher.finalize().to_vec()
    }
}

impl From<Block> for kv::Raw {
    fn from(block: Block) -> Self {
        let temp = serde_json::to_string(&block).expect("Block to kv::Raw error");
        let raw_data = temp.as_bytes();
        kv::Raw::from(raw_data)
    }
}

impl From<kv::Raw> for Block {
    fn from(raw: kv::Raw) -> Self {
        let json = String::from_utf8_lossy(&raw);
        serde_json::from_str(&json).expect("kv::Raw to Block error")
    }
}
