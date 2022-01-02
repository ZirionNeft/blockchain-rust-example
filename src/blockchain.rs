pub mod block {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use super::proof_of_work::ProofOfWork;

    type Payload = Value;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Block {
        pub index: u32,
        pub timestamp: String,
        pub payload: Payload,
        pub hash: String,
        pub prev_hash: String,
        pub nonce: u64,
    }

    impl Block {
        pub fn new(index: u32, prev_hash: String, payload: Value, timestamp: String) -> Block {
            let mut new_block: Block = Block {
                index,
                prev_hash,
                payload,
                timestamp,
                hash: "".to_string(),
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
}

pub mod proof_of_work {
    use std::cmp::Ordering;

    use num_bigint::BigUint;
    use sha2::{Digest, Sha256};

    use crate::blockchain::block;

    const TARGET_BITS: u16 = 18;
    const MAX_NONCE: u64 = u64::MAX;

    #[derive(Debug)]
    pub enum PowError {
        HashIsNotCreated,
    }

    pub struct ProofOfWork<'a> {
        block: &'a block::Block,
        target: BigUint,
    }

    impl<'a> ProofOfWork<'a> {
        pub fn new(block: &block::Block) -> ProofOfWork {
            let mut target = BigUint::new(vec![1]);

            if TARGET_BITS > 255 {
                panic!("TARGET_BITS must be lower than 256");
            }

            target <<= 256 - TARGET_BITS;

            println!(
                "PoW creating: \ntarget: {:#256b}\nTARGET_BITS: {}",
                target, TARGET_BITS
            );

            ProofOfWork { block, target }
        }

        pub fn run(&self) -> Result<(u64, String), PowError> {
            let mut hash_int: BigUint;
            let mut hash: Option<String> = None;
            let mut nonce = 0_u64;

            println!("Starting to mine the new block...");

            while nonce < MAX_NONCE {
                let data = self.prepare_data(nonce);
                let mut hasher = Sha256::new();
                hasher.update(data);

                let result = hasher.finalize();
                let hash_bytes: [u8; 32] = result.into();

                hash_int = BigUint::from_bytes_be(&hash_bytes);

                if hash_int.cmp(&self.target) == Ordering::Less {
                    hash = Some(format!("{:x}", result));
                    break;
                } else {
                    nonce += 1;
                };
            }

            match hash {
                Some(v) => {
                    println!("Block found: (nonce, hash) = ({}, {})", nonce, v);
                    Ok((nonce, v))
                }
                None => Err(PowError::HashIsNotCreated),
            }
        }

        pub fn validate(&self) -> bool {
            let data = self.prepare_data(self.block.nonce);
            let mut hasher = Sha256::new();
            hasher.update(data);

            let hash_bytes: [u8; 32] = hasher.finalize().into();

            let hash_int = BigUint::from_bytes_be(&hash_bytes);

            hash_int.cmp(&self.target) == Ordering::Less
        }

        fn prepare_data(&self, nonce: u64) -> Vec<u8> {
            let data = [
                self.block.index.to_ne_bytes().as_slice(),
                self.block.prev_hash.as_bytes(),
                serde_json::to_string(&self.block.payload)
                    .expect("Block payload to string convertion failed")
                    .as_bytes(),
                self.block.timestamp.as_bytes(),
                TARGET_BITS.to_ne_bytes().as_slice(),
                nonce.to_ne_bytes().as_slice(),
            ]
            .concat();

            data
        }
    }
}

pub struct Blockchain {
    pub chain: Vec<block::Block>,
}

impl Blockchain {
    pub fn new(blocks: &[block::Block]) -> Blockchain {
        Blockchain {
            chain: Vec::from(blocks),
        }
    }

    pub fn validate_block(new_block: &block::Block, previous: &block::Block) -> bool {
        if previous.index + 1 != new_block.index {
            return false;
        }

        if previous.hash != new_block.prev_hash {
            return false;
        }

        true
    }
}
