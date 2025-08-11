#[macro_use]
extern crate log;
extern crate env_logger as logger;

use std::sync::mpsc;
use std::thread::spawn;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[tokio::main]
pub async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();
    let mut client = devconsole_client::DCClient::new("ws://127.0.0.1:9001")
        .await
        .unwrap();
    let a = client.listen(1).await;
    info!("Listening to channel 1: {:?}", a);
    loop {}
}
