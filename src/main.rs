use actix_web::web::Data;
use actix_web::{App, HttpServer};
use http::{add_chain_block, create_blockchain, get_blockchain, AppState};

use std::io;

mod blockchain;
mod http;
mod utils;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_state = Data::new(AppState {});

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_blockchain)
            .service(add_chain_block)
            .service(create_blockchain)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
