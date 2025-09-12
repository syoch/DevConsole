extern crate env_logger as logger;
extern crate log;

mod pkt_uart;

use std::collections::HashMap;

use devconsole::{ChannelID, DCClient};
use devconsole_serial_protocol::SerialEvent;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::{spawn, sync::mpsc::{self, Sender}};

use crate::pkt_uart::{PktUARTRx, PktUARTTx};

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    // from_channel --> to_channel
    src: ChannelID,
    dst: ChannelID,
}

async fn split_rx_bytes(rx: mpsc::Receiver<Vec<u8>>) -> mpsc::Receiver<u8> {
    let (tx_byte, rx_byte) = mpsc::channel(100);
    spawn(async move {
        let mut rx = rx;
        let tx = tx_byte;
        while let Some(data) = rx.recv().await {
            for byte in data {
                if tx.send(byte).await.is_err() {
                    break;
                }
            }
        }
    });
    rx_byte
}

async fn pktuart_pipe_sd(rx_src: mpsc::Receiver<Vec<u8>>) {
    let mut pktuart = PktUARTRx::new(split_rx_bytes(rx_src).await);

    while let Some((addr, data)) = pktuart.read_pkt().await {
        debug!("Received packet from {addr}: {data:?}");
    }
}
async fn pktuart_pipe_ds(mut rx_dst: mpsc::Receiver<Vec<u8>>, tx: mpsc::Sender<u8>) {
    let tx = PktUARTTx::new(tx.clone());
    while let Some(data) = rx_dst.recv().await {
        let addr = data[0];
        let payload = data[1..].to_vec();

        tx.send(addr, payload).await;
    }
}

#[tokio::main]
pub async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    let mut client = DCClient::new("ws://127.0.0.1:9001").await.unwrap();

    let (tx, mut rx) = mpsc::channel(100);

    let ctrl_cid = client.open("PktUART".to_string()).await.unwrap();
    client.listen(ctrl_cid, tx.clone(), None).await.unwrap();

    while let Some((from, message)) = rx.recv().await {
        debug!("Received message from {from}: {message}");

        if from != ctrl_cid {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<Request>(&message) {
            debug!("Parsed request: {req:?}");

            let (tx_src, rx_src) = mpsc::channel(100);
            let (tx_dst, rx_dst) = mpsc::channel(100);

            client.listen(req.src, tx_src, None).await.unwrap();
            client.listen(req.dst, tx_dst, None).await.unwrap();

            spawn(pktuart_pipe_sd(rx_src));
            spawn(pktuart_pipe_ds(rx_dst, client.get_tx().clone()));
        } else if let Ok(event) = serde_json::from_str::<SerialEvent>(&message) {
            debug!("Parsed serial event: {event:?}");
        } else {
            debug!("Unknown message format");
        }
    }
}
