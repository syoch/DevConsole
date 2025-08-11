#[macro_use]
extern crate log;
extern crate env_logger as logger;

#[tokio::main]
pub async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();
    let mut client = devconsole_client::DCClient::new("ws://127.0.0.1:9001")
        .await
        .unwrap();

    let mut listening_channels = vec![];

    loop {
        let channel_list = client.channel_list().await.unwrap();
        for channel in channel_list {
            if !listening_channels.contains(&channel) {
                client.listen(channel).await.unwrap();
                listening_channels.push(channel);
            }
        }
    }
}
