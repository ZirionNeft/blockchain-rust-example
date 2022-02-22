use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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

impl From<&[u8]> for HashHex {
    fn from(bytes: &[u8]) -> Self {
        HashHex(bytes.to_vec())
    }
}

impl HashHex {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}
