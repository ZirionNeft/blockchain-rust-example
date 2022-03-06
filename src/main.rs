use actix_web::web::Data;
use actix_web::{App, HttpServer};
use http::{
    add_chain_block, create_blockchain, get_balance, get_blockchain, get_wallets, new_wallet,
};
use store::AppStore;

use std::io;
use std::sync::{Arc, Mutex};

mod blockchain;
mod http;
mod store;
mod utils;

pub struct AppState {
    store: Arc<Mutex<AppStore>>,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_state = Data::new(AppState {
        store: AppStore::new(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_blockchain)
            .service(add_chain_block)
            .service(create_blockchain)
            .service(get_balance)
            .service(new_wallet)
            .service(get_wallets)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
