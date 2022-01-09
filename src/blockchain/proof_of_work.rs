use std::cmp::Ordering;

use num_bigint::BigUint;
use sha2::{Digest, Sha256};

use crate::blockchain::block;

use super::block::HashHex;

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
            "PoW creating: \ntarget: {:x}\nTARGET_BITS: {}",
            target, TARGET_BITS
        );

        ProofOfWork { block, target }
    }

    pub fn run(&self) -> Result<(u64, HashHex), PowError> {
        let mut hash_int: BigUint;
        let mut hash: Vec<u8> = vec![];
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
                hash = hash_bytes.to_vec();
                break;
            } else {
                nonce += 1;
            };
        }

        if !hash.is_empty() {
            let hash_hex = HashHex(hash);

            println!(
                "Block found: (nonce, hash) = ({}, {})",
                nonce,
                serde_json::to_string(&hash_hex).unwrap()
            );
            Ok((nonce, hash_hex))
        } else {
            Err(PowError::HashIsNotCreated)
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
            &self.block.prev_hash.0,
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
