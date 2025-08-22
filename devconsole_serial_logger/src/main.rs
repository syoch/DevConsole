use devconsole_client::DCClient;
use devconsole_serial_protocol::SerialEvent;
use tokio::sync::mpsc;

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

#[tokio::main]
pub async fn main() {
    let mut client = devconsole_client::DCClient::new("ws://127.0.0.1:9001")
        .await
        .unwrap();

    let (tx, mut rx) = mpsc::channel(100);

    while get_serial_monitor_cid(&mut client).await.is_none() {}

    let serial_monitor_cid = get_serial_monitor_cid(&mut client)
        .await
        .expect("Failed to get SerialMonitor channel ID");
    client
        .listen(serial_monitor_cid, tx)
        .await
        .expect("Failed to listen to SerialMonitor channel");

    while let Some((_nid, msg)) = rx.recv().await {
        match serde_json::from_str(&msg).unwrap() {
            SerialEvent::Opened { path } => {
                println!("{path}");
            }
            SerialEvent::Line { path, line } => {
                println!("{path}: {line}");
            }
            SerialEvent::Closed { path } => {
                println!("{path}");
            }
        }
    }
}
