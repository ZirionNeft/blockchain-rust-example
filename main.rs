use std::sync::{Mutex};
use std::{io};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::web::{Data, Json};
use actix_web::{HttpServer, App, get, Result, error, post};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use sha2::{Sha256, Digest};

type Payload = Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Block {
    index: u32,
    timestamp: String,
    payload: Payload,
    hash: String,
    prev_hash: String,
}

fn generate_hash(block: &Block) -> String {
    let mut hasher = Sha256::new();
    let data = block.index.to_string() + &block.timestamp + &serde_json::to_string(&block.payload).unwrap() + &block.prev_hash;

    hasher.update(data);

    return format!("{:X}", hasher.finalize());
}

fn validate_block(new_block: &Block, previous: &Block) -> bool {
    if previous.index + 1 != new_block.index {
        return false;
    }

    if previous.hash != new_block.prev_hash {
        return false;
    }

    if generate_hash(new_block) != new_block.hash {
        return false;
    }

    true
}

fn replace_chain(current_chain: &mut Vec<Block>, new_blocks: Vec<Block>) {
    if new_blocks.len() > current_chain.len() {
        *current_chain = new_blocks;
    }
}

struct AppState {
    blockchain: Mutex<Vec<Block>>,
}

#[derive(Deserialize, Clone)]
struct AddChainBlockQuery {
    payload: Value
}

#[get("/")]
async fn get_blockchain(data: Data<AppState>) -> Result<Json<Vec<Block>>> {
    let blocks = data.blockchain.lock().unwrap();
    
    // let mut val = Vec::new();
    // val.append();

    Ok(Json((*blocks).clone()))
}

#[post("/")]
async fn add_chain_block(data: Data<AppState>, query_params: Json<AddChainBlockQuery>) -> Result<Json<Block>> {
    let mut blockchain = data.blockchain.lock().unwrap();

    if blockchain.len() == 0 {
        return Err(error::ErrorInternalServerError("Blockchain is empty (no genesis block)"));
    }

    let mut new_block: Block = Block {
        index: blockchain.len() as u32,
        prev_hash: blockchain[blockchain.len() - 1].hash.to_string(),
        payload: json!(&query_params.payload),
        timestamp: get_current_time(),
        hash: "".to_string(),
    };

    new_block.hash = generate_hash(&new_block);

    if !validate_block(&new_block, &blockchain[blockchain.len() - 1]) {
        return Err(error::ErrorConflict("Block conflict"));
    }

    blockchain.push(new_block.clone());

    Ok(Json(new_block))
}

fn get_current_time() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis().to_string()
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let mut blockchain: Vec<Block> = Vec::new();

    let mut genesis_block = Block {
        index: 0,
        payload: json!({
            "username": "Nikita"
        }), 
        prev_hash: "".to_string(),
        timestamp: get_current_time(),
        hash: "".to_string(),
    };

    genesis_block.hash = generate_hash(&genesis_block);

    blockchain.push(genesis_block);

    let app_state = Data::new(
        AppState {
            blockchain: Mutex::new(blockchain)
        }
    );

    HttpServer::new(move || {
        App::new()
        .app_data(app_state.clone())
            .service(get_blockchain)
            .service(add_chain_block)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}