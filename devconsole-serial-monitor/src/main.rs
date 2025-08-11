#[macro_use]
extern crate log;
extern crate env_logger as logger;

mod device_watcher;
mod serial_monitor;

use std::sync::mpsc;
use std::thread::spawn;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
enum SerialEvent {
    Opened { path: String },
    Line { path: String, line: String },
    Closed { path: String },
}

pub fn main1() {
    // initialize the logger as debug level
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    let (tx, rx) = mpsc::channel();

    info!("Starting serial device watcher...");
    spawn(move || {
        device_watcher::watcher_thread(tx);
    });

    let (serial_tx, serial_rx) = mpsc::channel();

    loop {
        let msg = rx.recv_timeout(Duration::from_millis(1));
        match msg {
            Ok(device_watcher::Event::DeviceFound(device)) => {
                info!("Found serial device: {}", device);
                let tx = serial_tx.clone();
                spawn(move || {
                    serial_monitor::monitor_thread(device, tx);
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(e) => {
                error!("Error receiving message from device watcher: {}", e);
                break;
            }
        }

        match serial_rx.recv_timeout(Duration::from_millis(1)) {
            Ok(serial_monitor::Event::LineReceipt(path, line)) => {
                info!("Received line from {}: {}", path, line);
            }
            Ok(serial_monitor::Event::Closed(path)) => {
                info!("Serial device closed: {}", path);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(e) => {
                error!("Error receiving message from serial device: {}", e);
                return;
            }
        }
    }
}

#[tokio::main]
pub async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();
    let mut client = devconsole_client::DCClient::new("ws://127.0.0.1:9001")
        .await
        .unwrap();
    let a = client
        .open("my_channel".to_string())
        .await
        .expect("Failed to open channel");
    info!("Opened channel: {:?}", a);

    loop {
        let b = client.send(a, "Hello, world!".to_string()).await;
        info!("Sent message: {:?}", b);

        sleep(Duration::from_secs(1)).await;
    }
}
