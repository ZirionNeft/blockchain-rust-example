use std::fmt::Formatter;
use std::{fmt::Display};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Sha256, Digest};

struct Block<'a, Payload: Display> {
    index: u32,
    timestamp: &'a str,
    payload: Payload,
    hash: String,
    prev_hash: &'static str,
}

fn generate_hash<Payload: Display>(block: &Block<Payload>) -> String {
    let mut hasher = Sha256::new();
    let data = block.index.to_string() + block.timestamp + &block.payload.to_string() + block.prev_hash;

    println!("data is {}", &data);

    hasher.update(data);

    return format!("{:X}", hasher.finalize());
}

struct TestPayload {
    username: &'static str,
}

impl Display for TestPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.username)
    }
}

fn main() {
    let mut genesis_block = Block {
        index: 0,
        payload: TestPayload {
            username: "Nikita"
        },
        prev_hash: "",
        timestamp: &SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis().to_string(),
        hash: "".to_string(),
    };

    genesis_block.hash = generate_hash(&genesis_block);
}