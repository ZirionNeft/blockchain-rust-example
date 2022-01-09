use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_with::serde_as;

use super::proof_of_work::ProofOfWork;

type Payload = JsonValue;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HashHex(#[serde_as(as = "serde_with::hex::Hex")] pub Vec<u8>);

impl From<Vec<u8>> for HashHex {
    fn from(vec: Vec<u8>) -> Self {
        HashHex(vec)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub timestamp: String,
    pub payload: Payload,
    pub hash: HashHex,
    pub prev_hash: HashHex,
    pub nonce: u64,
}

impl Block {
    pub fn new(prev_hash: HashHex, payload: JsonValue, timestamp: String) -> Block {
        let mut new_block: Block = Block {
            prev_hash,
            payload,
            timestamp,
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
