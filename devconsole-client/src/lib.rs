use std::{collections::HashMap, sync::Arc};

use devconsole_protocol::{ChannelID, Event};
use futures_util::{
    SinkExt, StreamExt,
    future::ready,
    stream::{SplitSink, SplitStream},
};
use log::{info, warn};
use tokio::{
    net::TcpStream,
    sync::{Mutex, oneshot},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, Message},
};

extern crate env_logger as logger;
extern crate log;

#[derive(Eq, Hash, PartialEq)]
enum DispatchID {
    Listen(ChannelID),
    ChannelList,
    ChannelInfo(ChannelID),
    None,
}

impl From<&Event> for DispatchID {
    fn from(event: &Event) -> Self {
        match event {
            Event::ChannelListenRequest { channel } => DispatchID::Listen(*channel),
            Event::ChannelListenResponse { channel, .. } => DispatchID::Listen(*channel),
            Event::ChannelListRequest => DispatchID::ChannelList,
            Event::ChannelListResponse { .. } => DispatchID::ChannelList,
            Event::ChannelInfoRequest(channel) => DispatchID::ChannelInfo(*channel),
            Event::ChannelInfoResponse { channel, .. } => DispatchID::ChannelInfo(*channel),

            Event::Data { channel, .. } => {
                panic!("Data event should not be dispatched: {:?}", channel)
            }

            Event::NodeIDNotification { node_id: _ }
            | Event::ChannelOpenRequest { name: _ }
            | Event::ChannelCloseRequest { channel: _ } => DispatchID::None,
        }
    }
}

type SharedDispatchers = Arc<Mutex<HashMap<DispatchID, oneshot::Sender<bool>>>>;

#[derive(Debug)]
pub enum DCClientError {
    WSError(tungstenite::Error),
    ConnectionBroken,
}

pub struct DCClient {
    tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    dispatches: SharedDispatchers,
    listening_channels: Vec<ChannelID>,
}

impl DCClient {
    pub async fn new(url: &str) -> Result<Self, tokio_tungstenite::tungstenite::Error> {
        let (ws_stream, _) = connect_async(url).await?;
        let (t, r) = ws_stream.split();
        let client = DCClient {
            tx: t,
            dispatches: Arc::new(Mutex::new(HashMap::new())),
            listening_channels: Vec::new(),
        };

        let dispatchers = client.dispatches.clone();
        tokio::spawn(async move {
            DCClient::thread(dispatchers, r).await;
        });

        Ok(client)
    }

    pub async fn listen(&mut self, channel: ChannelID) -> Result<(), DCClientError> {
        let (tx, rx) = oneshot::channel();
        if self.listening_channels.contains(&channel) {
            warn!("Channel {} is already being listened to", channel);
            return Ok(());
        }
        self.listening_channels.push(channel);

        self.send_evt(Event::ChannelListenRequest { channel })
            .await
            .map_err(DCClientError::WSError)?;
        self.dispatches
            .lock()
            .await
            .insert(DispatchID::Listen(channel), tx);

        let response = rx
            .await
            .expect("Failed to receive response for channel listen request");

        if response == false {
            Err(DCClientError::ConnectionBroken)
        } else {
            Ok(())
        }
    }

    async fn send_evt(
        &mut self,
        event: Event,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        let msg = serde_json::to_string(&event).unwrap();
        self.tx.send(Message::Text(msg.into())).await?;
        Ok(())
    }

    async fn thread(
        dispatchers: SharedDispatchers,
        r: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) {
        r.filter_map(|msg| async { msg.ok() })
            .filter_map(|x| ready(x.to_text().ok().map(|x| x.to_string())))
            .filter_map(|x| ready(serde_json::from_str::<Event>(&x).ok()))
            .for_each(|event| async {
                match event {
                    Event::NodeIDNotification { node_id } => {
                        info!("Node ID: {}", node_id);
                    }
                    Event::Data { channel, data } => {
                        info!("Received data on channel {}: {}", channel, data);
                    }
                    Event::ChannelOpenRequest { name } => {
                        info!("Channel open request for: {}", name);
                    }
                    Event::ChannelListenResponse { channel, success } => {
                        if let Some(tx) = dispatchers
                            .lock()
                            .await
                            .remove(&DispatchID::Listen(channel))
                        {
                            let _ = tx.send(success);
                        } else {
                            warn!(
                                "No dispatcher found for channel listen response: {}",
                                channel
                            );
                        }
                    }
                    // Event::ChannelListResponse { channels } => {}
                    // Event::ChannelInfoResponse {
                    //     channel,
                    //     name,
                    //     supplied_by,
                    // } => {}
                    _ => {
                        warn!("Unhandled event: {:?}", event);
                    }
                }
            })
            .await;
    }
}
