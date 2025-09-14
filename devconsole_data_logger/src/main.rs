use std::time::Duration;

use tokio::{spawn, sync::mpsc, time::sleep};

#[macro_use]
extern crate log;
extern crate env_logger as logger;

#[tokio::main]
pub async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();
    let mut client = devconsole::DCClient::new("ws://127.0.0.1:9001")
        .await
        .unwrap();

    let mut listening_channels = vec![];

    let (tx, mut rx) = mpsc::channel(64);
    let (tx_bin, mut rx_bin) = mpsc::channel(64);
    spawn(async move {
        while let Some((channel, data)) = rx.recv().await {
            info!("Received data on channel {channel}: {data}");
        }
    });
    spawn(async move {
        while let Some((channel, data)) = rx_bin.recv().await {
            info!("Received binary data on channel {channel}: {data:?}");
        }
    });

    loop {
        let channel_list = client.channel_list().await.unwrap();
        for channel in channel_list {
            if !listening_channels.contains(&channel) {
                client.listen(channel, Some(tx.clone()), Some(tx_bin.clone())).await.unwrap();
                listening_channels.push(channel);
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
