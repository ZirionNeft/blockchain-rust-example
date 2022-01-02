use actix_web::web::{Data, Json};
use actix_web::{error, get, post, App, HttpServer, Result};
use blockchain::Blockchain;
use serde::Deserialize;
use serde_json::{json, Value};

use std::io;
use std::sync::Mutex;

mod blockchain;
mod utils;

use blockchain::block::Block;
use utils::get_current_time;

struct AppState {
    blockchain: Mutex<Blockchain>,
}

#[derive(Deserialize)]
struct AddChainBlockQuery {
    payload: Value,
}

#[get("/")]
async fn get_blockchain(data: Data<AppState>) -> Result<Json<Vec<Block>>> {
    let blocks = data.blockchain.lock().unwrap();

    Ok(Json((*blocks).chain.clone()))
}

#[post("/")]
async fn add_chain_block(
    data: Data<AppState>,
    query_params: Json<AddChainBlockQuery>,
) -> Result<Json<Block>> {
    let mut blockchain = data.blockchain.lock().unwrap();

    if blockchain.chain.is_empty() {
        return Err(error::ErrorInternalServerError(
            "Blockchain is empty (no genesis block)",
        ));
    }

    let new_block: Block = Block::new(
        blockchain.chain.len() as u32,
        blockchain.chain[blockchain.chain.len() - 1]
            .hash
            .to_string(),
        json!(&query_params.payload),
        utils::get_current_time(),
    );

    if !blockchain::Blockchain::validate_block(
        &new_block,
        &blockchain.chain[blockchain.chain.len() - 1],
    ) {
        return Err(error::ErrorConflict("Block conflict"));
    }

    blockchain.chain.push(new_block.clone());

    Ok(Json(new_block))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let blockchain: Blockchain;

    let genesis_block = Block::new(
        0,
        "".to_string(),
        json!({
            "description": "This is a genesis block"
        }),
        get_current_time(),
    );

    blockchain = blockchain::Blockchain::new(&[genesis_block]);

    let app_state = Data::new(AppState {
        blockchain: Mutex::new(blockchain),
    });

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
