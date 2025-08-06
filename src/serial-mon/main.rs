#[macro_use]
extern crate log;
extern crate env_logger as logger;

mod device_watcher;
mod serial_monitor;

use std::sync::mpsc;
use std::thread::spawn;

pub fn main() {
    // initialize the logger as debug level
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    let (tx, rx) = mpsc::channel();

    info!("Starting serial device watcher...");
    spawn(move || {
        device_watcher::watcher_thread(tx);
    });

    // Keep the main thread alive to allow the watcher to run
    loop {
        let msg = rx.recv();
        match msg {
            Ok(device_watcher::Event::DeviceFound(device)) => {
                info!("Device found: /dev/{}", device);
            }
            Ok(device_watcher::Event::DeviceRemoved(device)) => {
                info!("Device removed: /dev/{}", device);
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                break; // Exit the loop on error
            }
        }
    }
}
