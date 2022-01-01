pub mod block {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use sha2::{Digest, Sha256};

    type Payload = Value;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Block {
        pub index: u32,
        pub timestamp: String,
        pub payload: Payload,
        pub hash: String,
        pub prev_hash: String,
    }

    impl Block {
        pub fn new(index: u32, prev_hash: String, payload: Value, timestamp: String) -> Block {
            let mut new_block: Block = Block {
                index,
                prev_hash,
                payload,
                timestamp,
                hash: "".to_string(),
            };

            new_block.hash = generate_hash(&new_block);

            new_block
        }
    }

    pub fn generate_hash(block: &Block) -> String {
        let mut hasher = Sha256::new();
        let data = block.index.to_string()
            + &block.timestamp
            + &serde_json::to_string(&block.payload).unwrap()
            + &block.prev_hash;

        hasher.update(data);

        return format!("{:X}", hasher.finalize());
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

        if block::generate_hash(new_block) != new_block.hash {
            return false;
        }

        true
    }

    pub fn replace_chain(&mut self, new_blocks: Vec<block::Block>) {
        if new_blocks.len() > self.chain.len() {
            self.chain = new_blocks;
        }
    }
}
