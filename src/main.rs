use std::{env, io};

use dotenv::dotenv;

use crate::switch::SwitchService;

mod switch;
mod server;

#[actix_rt::main]
async fn main() -> io::Result<()> {
    match dotenv() {
        Err(err) if !err.not_found() => panic!("Failed to load .env files: {}", err),
        Ok(path) => println!("Loaded environment from {:?}", path),
        _ => {}
    }
    pretty_env_logger::init();

    let scan_path = env::var("SWITCH_DIR").expect("SWITCH_DIR not set");
    let switch_service = SwitchService::new(&scan_path);

    server::start(switch_service).await
}
