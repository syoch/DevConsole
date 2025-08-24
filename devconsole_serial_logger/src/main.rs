extern crate env_logger as logger;
extern crate log;

mod pkt_uart;

use std::collections::HashMap;

use devconsole::DCClient;
use devconsole_serial_protocol::SerialEvent;
use log::debug;
use tokio::{spawn, sync::mpsc};

async fn get_serial_monitor_cid(client: &mut DCClient) -> Option<u64> {
    let channel_list = client
        .channel_list()
        .await
        .expect("Failed to get channel list");

    for cid in channel_list {
        let info = client
            .channel_info(cid)
            .await
            .expect("Failed to get channel info");
        if info.name == "SerialMonitor" {
            return Some(cid);
        }
    }

    None
}

struct DeviceHandler {
    tx: mpsc::Sender<u8>,
}

impl DeviceHandler {
    pub async fn new() -> Self {
        let (tx, rx) = mpsc::channel(32);
        spawn(DeviceHandler::task(rx));
        Self { tx }
    }

    async fn task(rx: mpsc::Receiver<u8>) {
        let mut rx = pkt_uart::PktUARTRx::new(rx);

        loop {
            let (dest_addr, data) = rx.read_pkt().await.expect("Failed to read packet");

            // Show
            debug!("Received packet: dest_addr={}, data={:?}", dest_addr, data);
        }
    }
}

struct Handler {
    devices: HashMap<String, DeviceHandler>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub async fn add_device(&mut self, name: String) {
        debug!("Adding device: {name}");
        self.devices.insert(name, DeviceHandler::new().await);
    }

    pub async fn add_byte(&mut self, name: String, byte: u8) {
        if let Some(handler) = self.devices.get_mut(&name) {
            handler.tx.send(byte).await.unwrap();
        }
    }

    pub fn remove_device(&mut self, device_name: &str) {
        debug!("Removing device: {device_name}");
        self.devices.remove(device_name);
    }

    pub fn has_device(&self, device_name: &str) -> bool {
        self.devices.contains_key(device_name)
    }
}

#[tokio::main]
pub async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    let mut client = DCClient::new("ws://127.0.0.1:9001").await.unwrap();

    let (tx, mut rx) = mpsc::channel(100);

    while get_serial_monitor_cid(&mut client).await.is_none() {}

    let serial_monitor_cid = get_serial_monitor_cid(&mut client)
        .await
        .expect("Failed to get SerialMonitor channel ID");

    client
        .listen(serial_monitor_cid, tx)
        .await
        .expect("Failed to listen to SerialMonitor channel");

    let mut handler = Handler::new();

    while let Some((_nid, msg)) = rx.recv().await {
        match serde_json::from_str(&msg).unwrap() {
            SerialEvent::Opened { path } => {
                handler.add_device(path).await;
            }
            SerialEvent::Line { path, line: data } => {
                if !handler.has_device(&path) {
                    handler.add_device(path.clone()).await;
                }

                for byte in data {
                    handler.add_byte(path.clone(), byte).await;
                }
            }
            SerialEvent::Closed { path } => {
                handler.remove_device(&path);
            }
        }
    }
}
