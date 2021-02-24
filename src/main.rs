use std::env;

use dotenv::dotenv;

use crate::switch::SwitchService;

mod server;
mod switch;

#[tokio::main]
async fn main() {
    match dotenv() {
        Err(err) if !err.not_found() => panic!("Failed to load .env files: {}", err),
        Ok(path) => println!("Loaded environment from {:?}", path),
        _ => {}
    }

    let scan_path = env::var("SWITCH_DIR").expect("SWITCH_DIR not set");
    let switch_service = SwitchService::new(&scan_path);

    server::start(switch_service).await
}
