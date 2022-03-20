use std::sync::Arc;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::blockchain::wallet::Wallet;
use crate::blockchain::Blockchain;
use crate::utils::HashHex;
use crate::AppState;
use actix_web::web::{Data, Json, Path};
use actix_web::{error, get, post, Responder, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SendBody {
    from: String,
    to: String,
    amount: i32,
}

#[derive(Serialize)]
pub struct GetBalanceReponse {
    balance: u32,
}

#[derive(Deserialize)]
pub struct CreateBlockchainBody {
    address: String,
}

#[derive(Serialize)]
pub struct CreateWalletResponse {
    wallet_address: HashHex,
}

#[get("/")]
pub async fn get_blockchain(state: Data<AppState>) -> Result<Json<Vec<Block>>> {
    let store = Arc::clone(&state.store);
    let store = store.lock().unwrap();

    if !Blockchain::exists(&store) {
        return Err(error::ErrorNotFound("Blockchain not initialized yet"));
    }

    let blockchain = match Blockchain::new(None, &store) {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };

    let buffer: Vec<Block> = blockchain.into_iter().collect();

    Ok(Json(buffer))
}

#[post("/")]
pub async fn create_blockchain(
    state: Data<AppState>,
    body: Json<CreateBlockchainBody>,
) -> Result<Json<Vec<Block>>> {
    let store = Arc::clone(&state.store);
    let store = store.lock().unwrap();

    if Blockchain::exists(&store) {
        return Err(error::ErrorConflict("Blockchain already exists"));
    }

    let address = body.address.to_owned();

    let blockchain =
        Blockchain::new(Some(address), &store).map_err(error::ErrorInternalServerError)?;

    let buffer: Vec<Block> = blockchain.into_iter().collect();

    Ok(Json(buffer))
}

#[get("/coins/{address}")]
pub async fn get_balance(state: Data<AppState>, path: Path<(String,)>) -> Result<impl Responder> {
    let store = Arc::clone(&state.store);
    let store = store.lock().unwrap();

    if !Blockchain::exists(&store) {
        return Err(error::ErrorNotFound("Blockchain not initialized yet"));
    }

    let address = path.into_inner().0;

    let blockchain =
        Blockchain::new(Some(address.clone()), &store).map_err(error::ErrorInternalServerError)?;

    let wallet = match Wallet::get_by(&address, &store) {
        Some(v) => v,
        None => return Err(error::ErrorNotFound("Wallet not found")),
    };

    let pub_key: Vec<u8> = wallet.pub_key_bytes_vec();

    let utxo = blockchain.find_utxo(&Wallet::hash_pub_key(pub_key));

    let balance = utxo.iter().fold(0, |acc, out| acc + out.value);

    Ok(Json(GetBalanceReponse { balance }))
}

#[post("/wallet")]
pub async fn new_wallet(state: Data<AppState>) -> Result<Json<CreateWalletResponse>> {
    let store = Arc::clone(&state.store);
    let wallet_address = Wallet::create(store).map_err(error::ErrorInternalServerError)?;

    Ok(Json(CreateWalletResponse { wallet_address }))
}

#[get("/wallet")]
pub async fn get_wallets(state: Data<AppState>) -> Result<Json<Vec<HashHex>>> {
    let store = Arc::clone(&state.store);
    let store = store.lock().unwrap();

    let wallet_address =
        Wallet::get_all_addresses(&store).map_err(error::ErrorInternalServerError)?;

    Ok(Json(wallet_address))
}

#[post("/coins")]
pub async fn add_chain_block(state: Data<AppState>, body: Json<SendBody>) -> Result<Json<Block>> {
    let store = Arc::clone(&state.store);
    let store = store.lock().unwrap();

    let from = body.from.clone();

    let mut blockchain = Blockchain::new(Some(from), &store).unwrap(); //.map_err(error::ErrorInternalServerError)?;

    if blockchain.tip.0.is_empty() {
        return Err(error::ErrorInternalServerError("Blockchain tip is empty"));
    }

    if body.amount <= 0 {
        return Err(error::ErrorBadRequest(
            "Amount value can't be low or equal than zero",
        ));
    }

    if body.from == body.to {
        return Err(error::ErrorBadRequest("You can't send money to yourself"));
    }

    let transaction = Transaction::new_utxo(
        body.from.to_owned(),
        body.to.to_owned(),
        body.amount as u32,
        &blockchain,
    )
    .map_err(error::ErrorInternalServerError)?;

    let added_block = blockchain
        .add_block(vec![transaction])
        .map_err(error::ErrorInternalServerError)?;

    Ok(Json(added_block))
}
