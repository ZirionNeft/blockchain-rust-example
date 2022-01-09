use actix_web::web::{Data, Json};
use actix_web::{error, get, post, App, HttpServer, Result};
use blockchain::Blockchain;
use serde::Deserialize;
use serde_json::Value;

use std::io;
use std::sync::Mutex;

use crate::blockchain::block::Block;

mod blockchain;
mod utils;

struct AppState<'a> {
    blockchain: Mutex<Blockchain<'a>>,
}

#[derive(Deserialize)]
struct AddChainBlockQuery {
    payload: Value,
}

#[get("/")]
async fn get_blockchain(data: Data<AppState<'_>>) -> Result<Json<Vec<Block>>> {
    let blockchain = data.blockchain.lock().unwrap();

    let mut buffer: Vec<Block> = vec![];
    for block in blockchain.clone() {
        buffer.push(block);
    }

    Ok(Json(buffer))
}

#[post("/")]
async fn add_chain_block(
    data: Data<AppState<'_>>,
    query_params: Json<AddChainBlockQuery>,
) -> Result<Json<Block>> {
    let mut blockchain = data.blockchain.lock().unwrap();

    if blockchain.tip.0.is_empty() {
        return Err(error::ErrorInternalServerError("Blockchain tip is empty"));
    }

    let added_block = match blockchain.add_block(query_params.payload.clone()) {
        Ok(v) => v,
        Err(e) => return Err(error::ErrorInternalServerError(e)),
    };

    Ok(Json(added_block))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let blockchain: Blockchain;

    blockchain = match blockchain::Blockchain::new() {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };

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
