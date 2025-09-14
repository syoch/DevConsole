extern crate env_logger as logger;
extern crate log;

mod pkt_uart;


use devconsole::{ChannelID, DCClient};
use devconsole_serial_protocol::SerialEvent;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::{
    select, spawn, sync::mpsc::{self}
};

use crate::pkt_uart::{PktUARTRx, PktUARTTx};

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    // from_channel --> to_channel
    src: ChannelID,
    dst_ch_name: String,
}

#[derive(Debug)]
struct DCSendRequest {
    dst: ChannelID,
    data: Vec<u8>,
}

#[derive(Debug)]
struct DCTxPort {
    tx: mpsc::Sender<DCSendRequest>,
    channel_id: ChannelID,
}

impl DCTxPort {
    pub async fn send(&self, data: Vec<u8>) {
        let req = DCSendRequest {
            dst: self.channel_id,
            data,
        };
        if let Err(e) = self.tx.send(req).await {
            debug!("Failed to send data to DCTxPort: {e}");
        }
    }

    pub fn to_mpsc(&self) -> mpsc::Sender<Vec<u8>> {
        let (tx, mut rx) = mpsc::channel(100);
        let tx_dc = self.tx.clone();
        let dst = self.channel_id;
        spawn(async move {
            while let Some(data) = rx.recv().await {
                let req = DCSendRequest { dst, data };
                if let Err(e) = tx_dc.send(req).await {
                    debug!("Failed to send data to DC: {e}");
                }
            }
        });
        tx
    }
}

async fn split_rx_bytes(mut rx: mpsc::Receiver<(ChannelID, Vec<u8>)>) -> mpsc::Receiver<u8> {
    let (tx_byte, rx_byte) = mpsc::channel(100);
    spawn(async move {
        let tx = tx_byte;
        while let Some((_, data)) = rx.recv().await {
            for byte in data {
                if tx.send(byte).await.is_err() {
                    break;
                }
            }
        }
    });
    rx_byte
}

async fn pktuart_decoder(rx_src: mpsc::Receiver<(ChannelID, Vec<u8>)>, dc_tx_port: DCTxPort) {
    let mut pktuart = PktUARTRx::new(split_rx_bytes(rx_src).await);

    while let Some((addr, data)) = pktuart.read_pkt().await {
        let mut out_data = vec![addr];
        out_data.extend(data);
        dc_tx_port.send(out_data).await;
    }
}
async fn pktuart_encoder(mut rx_dst: mpsc::Receiver<(ChannelID, Vec<u8>)>, dc_tx_port: DCTxPort) {
    let tx = PktUARTTx::new(dc_tx_port.to_mpsc());
    while let Some((_, data)) = rx_dst.recv().await {
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
    client
        .listen(ctrl_cid, Some(tx.clone()), None)
        .await
        .unwrap();
    let (dc_tx, mut dc_rx) = mpsc::channel(100);

    loop {
        select! {
            a = rx.recv() => {
                let (channel, message) = a.unwrap();

                debug!("Received message from {message} on channel {channel}");

                if let Ok(req) = serde_json::from_str::<Request>(&message) {
                    debug!("Parsed request: {req:?}");

                    let dst_ch_id = client.open(req.dst_ch_name.clone()).await.unwrap();

                    let (tx_src, rx_src) = mpsc::channel(100);
                    let (tx_dst, rx_dst) = mpsc::channel(100);

                    client.listen(req.src, None, Some(tx_src)).await.unwrap();
                    client.listen(dst_ch_id, None, Some(tx_dst)).await.unwrap();

                    spawn(pktuart_decoder(rx_src, DCTxPort {
                        tx: dc_tx.clone(),
                        channel_id: dst_ch_id,
                    }));
                    spawn(pktuart_encoder(rx_dst, DCTxPort {
                        tx: dc_tx.clone(),
                        channel_id: req.src,
                    }));
                } else if let Ok(event) = serde_json::from_str::<SerialEvent>(&message) {
                    debug!("Parsed serial event: {event:?}");
                } else {
                    debug!("Unknown message format");
                }
            }

            b = dc_rx.recv() => {
                let req = b.unwrap();
                client.send_bin(req.dst, req.data).await.unwrap();
            }


        }
    }
}
