use std::time::{SystemTime, UNIX_EPOCH};

// use p256::ecdsa::VerifyingKey;
use ripemd::Ripemd160;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sha2::{Digest, Sha256};

pub fn get_current_time() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        .to_string()
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct HashHex(#[serde_as(as = "serde_with::hex::Hex")] pub Vec<u8>);

impl From<Vec<u8>> for HashHex {
    fn from(vec: Vec<u8>) -> Self {
        HashHex(vec)
    }
}

impl From<HashHex> for Vec<u8> {
    fn from(hash_hex: HashHex) -> Self {
        hash_hex.0
    }
}

impl HashHex {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0
    }
}

pub fn checksum_hash(payload: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let result = hasher.finalize();

    let mut hasher = Sha256::new();
    hasher.update(result);
    hasher.finalize().to_vec()
}
