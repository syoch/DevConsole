#[macro_use]
extern crate log;
extern crate env_logger as logger;

mod device_watcher;
mod serial_monitor;

use devconsole_serial_protocol::SerialEvent;
use std::sync::mpsc;
use std::thread::spawn;
use std::time::Duration;

fn monitor(event_tx: mpsc::Sender<SerialEvent>) {
    let (dev_tx, dev_rx) = mpsc::channel();
    spawn(move || {
        device_watcher::watcher_thread(dev_tx);
    });

    let (serial_tx, serial_rx) = mpsc::channel();
    loop {
        let msg = dev_rx.recv_timeout(Duration::from_millis(1));
        // debug!("Received message from device watcher: {:?}", msg);
        match msg {
            Ok(device_watcher::Event::DeviceFound(device)) => {
                event_tx
                    .send(SerialEvent::Opened {
                        path: device.clone(),
                    })
                    .unwrap();

                let tx = serial_tx.clone();
                spawn(move || {
                    serial_monitor::monitor_thread(device, tx);
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(e) => {
                error!("Error receiving message from device watcher: {e}");
                break;
            }
        }

        let msg = serial_rx.recv_timeout(Duration::from_millis(1));
        // debug!("Received message from serial monitor: {:?}", msg);
        match msg {
            Ok(serial_monitor::Event::LineReceipt(path, line)) => {
                event_tx
                    .send(SerialEvent::Line {
                        path,
                        line: line.to_string(),
                    })
                    .unwrap();
            }
            Ok(serial_monitor::Event::Closed(path)) => {
                event_tx.send(SerialEvent::Closed { path }).unwrap();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(e) => {
                error!("Error receiving message from serial device: {e}");
                return;
            }
        }
    }
}

#[tokio::main]
pub async fn main() {
    // initialize the logger as debug level
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    let mut client = devconsole_client::DCClient::new("ws://localhost:9001")
        .await
        .expect("Failed to connect to WebSocket server");

    let channel = client
        .open("SerialMonitor".to_string())
        .await
        .expect("Failed to open channel");

    let (event_tx, event_rx) = mpsc::channel();
    spawn(move || {
        monitor(event_tx);
    });

    loop {
        let event = event_rx.recv().expect("Failed to receive event");
        let payload = serde_json::to_string(&event).unwrap();

        client.send(channel, payload).await.unwrap();
    }
}
