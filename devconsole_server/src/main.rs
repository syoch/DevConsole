extern crate env_logger as logger;
extern crate log;

mod channel;
mod client;
mod id_manager;
mod server;

use devconsole::{ChannelInfo, Event};
use futures_util::StreamExt;
use log::{error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;

use crate::{client::SharedClient, server::SharedServer};

async fn client_handler(stream: TcpStream, server: SharedServer) {
    let (writer, reader) = accept_async(stream)
        .await
        .expect("Error during WebSocket handshake")
        .split();

    let node_id = server.get_new_node_id().await;
    let client = SharedClient::new(writer, node_id);

    server.add_connection(client.clone()).await;

    client
        .send_event(Event::NodeIDNotification { node_id })
        .await
        .unwrap();

    let mut reader = reader
        .filter(|x| futures_util::future::ready(x.is_ok()))
        /* .inspect(|x| { info!("Received message: {:?}", x);}) */;

    loop {
        let msg = match reader.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(e)) => {
                error!("Error receiving message: {e}");
                break;
            }
            None => {
                break;
            }
        };

        if msg.is_binary() || msg.is_text() {
            let msg = msg.to_text().unwrap();
            let evt = serde_json::from_str::<Event>(msg).unwrap();
            // debug!("Received message: {evt:?}");

            match evt {
                Event::Data { channel, data } => {
                    server.broadcast_data(channel, data, client.node_id().await).await;
                }
                Event::DataBin { channel, data } => {
                    server.broadcast_bin_data(channel, data, client.node_id().await).await;
                }
                Event::ChannelOpenRequest { name } => {
                    let channel = server.new_channel(name, node_id).await;
                    client
                        .send_event(Event::ChannelOpenResponse {
                            channel,
                            success: true,
                        })
                        .await
                        .unwrap();
                }
                Event::ChannelListenRequest { channel } => {
                    let response = Event::ChannelListenResponse {
                        channel,
                        success: client.listen(channel).await.is_ok(),
                    };
                    client.send_event(response).await.unwrap();
                }
                Event::ChannelCloseRequest { channel } => {
                    info!("Received ChannelCloseRequest for channel {channel}");
                }

                Event::ChannelListRequest => {
                    let channels = server.get_channel_ids().await;
                    let response = Event::ChannelListResponse { channels };
                    client.send_event(response).await.unwrap();
                }

                Event::ChannelInfoRequest(channel) => {
                    if let Some(info) = server.get_channel(channel).await {
                        let channel_info = ChannelInfo {
                            channel,
                            name: info.name().to_string(),
                            supplied_by: info.supplied_by(),
                        };
                        let response = Event::ChannelInfoResponse(channel_info);
                        client.send_event(response).await.unwrap();
                    } else {
                        error!("ChannelInfoRequest for unknown channel {channel}");
                    }
                }

                _ => {
                    error!("Unhandled event: {evt:?}");
                }
            }
        }
    }

    server.remove_connection(&client).await;
}

#[tokio::main]
async fn main() {
    logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();
    let tcp_server = TcpListener::bind("127.0.0.1:9001").await.unwrap();

    let server: SharedServer = SharedServer::new_default();

    while let Ok((stream, _)) = tcp_server.accept().await {
        tokio::spawn(client_handler(stream, server.clone()));
    }
}
