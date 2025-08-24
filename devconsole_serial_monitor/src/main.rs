#[macro_use]
extern crate log;
extern crate env_logger as logger;

mod device_watcher;
mod serial_monitor;

use devconsole::DCClient;
use devconsole_serial_protocol::{SerialEvent, SerialRequest};
use std::collections::HashMap;
use tokio::{
    select, spawn,
    sync::mpsc::{self, Receiver, Sender},
};

enum MuxerRequest {
    NewPair(String),
}
enum MuxerResponse {
    NewPair(String, Receiver<serial_monitor::RequestToDevice>),
}

struct RequestMuxer {
    ctrl_req_tx: Sender<MuxerRequest>,
    ctrl_res_rx: Receiver<MuxerResponse>,
    data_tx: Sender<SerialRequest>,
}

impl RequestMuxer {
    pub fn new() -> Self {
        let (ctrl_req_tx, ctrl_req_rx) = mpsc::channel(64);
        let (ctrl_res_tx, ctrl_res_rx) = mpsc::channel(64);
        let (data_tx, data_rx) = mpsc::channel(64);
        let muxer = Self {
            ctrl_req_tx,
            ctrl_res_rx,
            data_tx,
        };
        {
            spawn(RequestMuxer::task(data_rx, ctrl_req_rx, ctrl_res_tx));
        }
        muxer
    }

    async fn task(
        mut data_rx: Receiver<SerialRequest>,
        mut ctrl_req_rx: Receiver<MuxerRequest>,
        ctrl_res_tx: Sender<MuxerResponse>,
    ) {
        let mut rxs = HashMap::new();
        loop {
            select! (
                request = ctrl_req_rx.recv() => {
                    match request {
                        Some(MuxerRequest::NewPair(path)) => {
                            let (tx, rx) = mpsc::channel(64);
                            rxs.insert(path.clone(), tx);
                            ctrl_res_tx.send(MuxerResponse::NewPair(path, rx)).await.expect("Failed to send new pair response");
                        }
                        _ => {}
                    }
                }
                request = data_rx.recv() => {
                    match request {
                        Some(SerialRequest::Data { path, data }) => {
                            if let Some(tx) = rxs.get(&path) {
                                tx.send(serial_monitor::RequestToDevice::Data(data))
                                    .await.expect("Failed to send data to device");
                            } else {
                                warn!("No TX found for path: {}", path);
                            }
                        }
                        _ => {}
                    }
                }
            )
        }
    }

    pub async fn add_tx(
        &mut self,
        path: &str,
    ) -> Option<mpsc::Receiver<serial_monitor::RequestToDevice>> {
        self.ctrl_req_tx
            .send(MuxerRequest::NewPair(path.to_string()))
            .await
            .expect("Failed to send new pair request");

        self.ctrl_res_rx.recv().await.and_then(|res| match res {
            MuxerResponse::NewPair(p, rx) if p == path => Some(rx),
            _ => None,
        })
    }
}

async fn monitor(event_tx: Sender<SerialEvent>, mut data_rx: Receiver<SerialRequest>) {
    let (dev_tx, mut dev_rx) = mpsc::channel(64);
    let (serial_tx, mut serial_rx) = mpsc::channel(64);
    spawn(device_watcher::watcher_thread(dev_tx));

    let mut req_muxer = RequestMuxer::new();

    loop {
        select! {
            msg = data_rx.recv() => {
                match msg {
                    Some(SerialRequest::Data { path, data }) => {
                        req_muxer
                            .data_tx
                            .send(SerialRequest::Data { path, data })
                            .await.expect("Failed to send data to muxer");
                    }
                    None => {
                        error!("Error receiving message from data channel");
                        break;
                    }
                }
            }

            msg = dev_rx.recv() => {
                match msg {
                    Some(device_watcher::Event::DeviceFound(device)) => {
                        event_tx
                            .send(SerialEvent::Opened {
                                path: device.clone(),
                            })
                            .await.expect("Failed to send opened event");

                        let tx = serial_tx.clone();

                        let req_rx = req_muxer
                            .add_tx(&device)
                            .await
                            .expect("Failed to add TX for device");
                        spawn(
                            serial_monitor::monitor_thread(device, tx, req_rx)
                        );
                    }
                    None => {
                        error!("Error receiving message from device watcher");
                        break;
                    }
                }
            }
            msg = serial_rx.recv() => {
                match msg {
                    Some(serial_monitor::Event::LineReceipt(path, line)) => {
                        event_tx
                            .send(SerialEvent::Line {
                                path,
                                line: line,
                            })
                            .await.expect("Failed to send line event");
                    }
                    Some(serial_monitor::Event::Closed(path)) => {
                        event_tx.send(SerialEvent::Closed { path }).await.expect("Failed to send closed event");
                    }
                    None => {
                        error!("Error receiving message from serial device");
                        return;
                    }
                }
            }
        }
    }
}

async fn outbound_transformer(
    mut outbound_rx: Receiver<(u64, String)>,
    req_tx: Sender<SerialRequest>,
) -> () {
    while let Some((_from, data)) = outbound_rx.recv().await {
        let msg = serde_json::from_str::<SerialRequest>(&data).expect("Failed to parse request");

        req_tx
            .send(msg)
            .await
            .map_err(|e| {
                error!("Failed to send request to monitor: {e}");
            })
            .ok();
    }
}

#[tokio::main]
pub async fn main() {
    // initialize the logger as debug level
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    let mut client = DCClient::new("ws://localhost:9001")
        .await
        .expect("Failed to connect to WebSocket server");

    let channel = client
        .open("SerialMonitor".to_string())
        .await
        .expect("Failed to open channel");

    let outbound_cid = client
        .open("SerialMonitor-Outbound".to_string())
        .await
        .expect("Failed to open channel");

    let (outbound_tx, outbound_rx) = mpsc::channel(64);
    client
        .listen(outbound_cid, outbound_tx)
        .await
        .expect("Failed to listen");
    let (req_tx, req_rx) = mpsc::channel(64);
    spawn(outbound_transformer(outbound_rx, req_tx));

    let (event_tx, mut event_rx) = mpsc::channel(64);
    spawn(monitor(event_tx, req_rx));

    loop {
        let event = event_rx.recv().await.expect("Failed to receive event");
        let payload = serde_json::to_string(&event).unwrap();

        client.send(channel, payload).await.unwrap();
    }
}
