use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::blockchain::wallet::Wallet;
use crate::blockchain::Blockchain;
use crate::utils::HashHex;
use actix_web::web::{Json, Path};
use actix_web::{error, get, post, Responder, Result};
use serde::{Deserialize, Serialize};

pub struct AppState {}

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
pub async fn get_blockchain() -> Result<Json<Vec<Block>>> {
    if !Blockchain::exists() {
        return Err(error::ErrorNotFound("Blockchain not initialized yet"));
    }

    let blockchain = match Blockchain::new(None) {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };

    let buffer: Vec<Block> = blockchain.into_iter().collect();

    Ok(Json(buffer))
}

#[post("/")]
pub async fn create_blockchain(body: Json<CreateBlockchainBody>) -> Result<Json<Vec<Block>>> {
    if Blockchain::exists() {
        return Err(error::ErrorConflict("Blockchain already exists"));
    }

    let address = body.address.to_owned();

    let blockchain = Blockchain::new(Some(address)).expect("Can't create blockchain");

    let buffer: Vec<Block> = blockchain.into_iter().collect();

    Ok(Json(buffer))
}

#[get("/coins/{address}")]
pub async fn get_balance(path: Path<(String,)>) -> Result<impl Responder> {
    if !Blockchain::exists() {
        return Err(error::ErrorNotFound("Blockchain not initialized yet"));
    }

    let address = path.into_inner().0;

    let blockchain = match Blockchain::new(Some(address.clone())) {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };

    let wallet = match Wallet::get_by(&address) {
        Some(v) => v,
        None => return Err(error::ErrorNotFound("Wallet not found")),
    };

    let pub_key: Vec<u8> = wallet.pub_key_bytes_vec();

    let utxo = blockchain.find_utxo(&Wallet::hash_pub_key(pub_key));

    let balance = utxo.iter().fold(0, |acc, out| acc + out.value);

    Ok(Json(GetBalanceReponse { balance }))
}

#[post("/wallet")]
pub async fn new_wallet() -> Result<Json<CreateWalletResponse>> {
    let wallet_address = Wallet::create().map_err(error::ErrorInternalServerError)?;

    Ok(Json(CreateWalletResponse { wallet_address }))
}

#[get("/wallet")]
pub async fn get_wallets() -> Result<Json<Vec<HashHex>>> {
    let wallet_address = Wallet::get_all_addresses().map_err(error::ErrorInternalServerError)?;

    Ok(Json(wallet_address))
}

#[post("/coins")]
pub async fn add_chain_block(body: Json<SendBody>) -> Result<Json<Block>> {
    let from = body.from.clone();
    let mut blockchain = match Blockchain::new(Some(from)) {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };

    if blockchain.tip.0.is_empty() {
        return Err(error::ErrorInternalServerError("Blockchain tip is empty"));
    }

    if body.amount <= 0 {
        return Err(error::ErrorBadRequest(
            "Amount value can't be low or equal than zero",
        ));
    }

    let transaction = Transaction::new_utxo(
        body.from.to_owned(),
        body.to.to_owned(),
        body.amount as u32,
        &blockchain,
    )
    .map_err(error::ErrorBadRequest)?;

    let added_block = match blockchain.add_block(vec![transaction]) {
        Ok(v) => v,
        Err(e) => return Err(error::ErrorInternalServerError(e)),
    };

    Ok(Json(added_block))
}
