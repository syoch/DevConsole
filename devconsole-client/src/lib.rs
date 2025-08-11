use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use devconsole_protocol::{ChannelID, Event};
use futures_util::{
    SinkExt, StreamExt,
    future::ready,
    stream::{SplitSink, SplitStream},
};
use log::{debug, info, warn};
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

#[derive(Eq, Hash, PartialEq, Debug)]
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
            | Event::ChannelOpenResponse {
                channel: _,
                success: _,
            }
            | Event::ChannelCloseRequest { channel: _ } => DispatchID::None,
        }
    }
}

#[derive(Default)]
struct Dispatchers {
    events: HashMap<DispatchID, oneshot::Sender<bool>>,
    resolve_channel: Option<oneshot::Sender<ChannelID>>,
}

struct SharedDispatchers(Arc<Mutex<Dispatchers>>);

impl Deref for SharedDispatchers {
    type Target = Arc<Mutex<Dispatchers>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedDispatchers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for SharedDispatchers {
    fn default() -> Self {
        SharedDispatchers(Arc::new(Mutex::new(Dispatchers::default())))
    }
}

impl Clone for SharedDispatchers {
    fn clone(&self) -> Self {
        SharedDispatchers(self.0.clone())
    }
}

impl SharedDispatchers {
    pub async fn wait_for_event(&self, id: DispatchID) -> bool {
        let (tx, rx) = oneshot::channel();

        self.lock().await.events.insert(id, tx);

        let response = rx
            .await
            .expect("Failed to receive response for channel listen request");

        response
    }

    pub async fn dispatch_event(&self, id: DispatchID, success: bool) {
        if let Some(tx) = self.lock().await.events.remove(&id) {
            let _ = tx.send(success);
        } else {
            warn!("No dispatcher found for event: {:?}", id);
        }
    }

    pub async fn wait_for_channel(&self) -> ChannelID {
        let (tx, rx) = oneshot::channel();
        self.lock().await.resolve_channel = Some(tx);

        rx.await.expect("Failed to receive channel ID")
    }

    pub async fn dispatch_channel(&self, channel: ChannelID) {
        if let Some(tx) = self.lock().await.resolve_channel.take() {
            let _ = tx.send(channel);
        } else {
            warn!("No dispatcher found for channel resolution");
        }
    }
}

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
            dispatches: SharedDispatchers::default(),
            listening_channels: Vec::new(),
        };

        let dispatchers = client.dispatches.clone();
        tokio::spawn(async move {
            DCClient::thread(dispatchers, r).await;
        });

        Ok(client)
    }

    pub async fn listen(&mut self, channel: ChannelID) -> Result<(), DCClientError> {
        if self.listening_channels.contains(&channel) {
            warn!("Channel {} is already being listened to", channel);
            return Ok(());
        }
        self.listening_channels.push(channel);

        self.send_evt(Event::ChannelListenRequest { channel })
            .await
            .map_err(DCClientError::WSError)?;

        let response = self
            .dispatches
            .wait_for_event(DispatchID::Listen(channel))
            .await;

        if response == false {
            Err(DCClientError::ConnectionBroken)
        } else {
            Ok(())
        }
    }

    pub async fn send(&mut self, channel: ChannelID, data: String) -> Result<(), DCClientError> {
        self.send_evt(Event::Data { channel, data })
            .await
            .map_err(DCClientError::WSError)
    }

    pub async fn open(&mut self, name: String) -> Result<ChannelID, DCClientError> {
        self.send_evt(Event::ChannelOpenRequest { name })
            .await
            .map_err(DCClientError::WSError)?;

        let channel = self.dispatches.wait_for_channel().await;
        Ok(channel)
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
                    Event::ChannelOpenResponse { channel, success } => {
                        info!("Channel open request for: {}", channel);
                        dispatchers.dispatch_channel(channel).await;
                    }
                    Event::ChannelListenResponse { channel, success } => {
                        info!("Channel listen response for {}: {}", channel, success);
                        dispatchers
                            .dispatch_event(DispatchID::Listen(channel), success)
                            .await;
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
