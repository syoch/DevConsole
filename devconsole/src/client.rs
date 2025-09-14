use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{ChannelID, ChannelInfo, Event, NodeID};
use futures_util::{
    SinkExt, StreamExt,
    future::ready,
    stream::{SplitSink, SplitStream},
};
use log::{info, warn};
use tokio::{
    net::TcpStream,
    sync::{Mutex, mpsc, oneshot},
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
            Event::ChannelInfoResponse(info) => DispatchID::ChannelInfo(info.channel),

            Event::Data { channel, .. } => {
                panic!("Data event should not be dispatched: {channel:?}")
            }
            Event::DataBin { channel, .. } => {
                panic!("DataBin event should not be dispatched: {channel:?}")
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
    channel_list: Option<oneshot::Sender<Vec<ChannelID>>>,
    channel_info: Option<oneshot::Sender<ChannelInfo>>,
    data_handlers: HashMap<ChannelID, mpsc::Sender<(ChannelID, String)>>,
    bin_data_handlers: HashMap<ChannelID, mpsc::Sender<(ChannelID, Vec<u8>)>>,
    node_id: Option<NodeID>,
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

        rx.await
            .expect("Failed to receive response for channel listen request")
    }

    pub async fn dispatch_event(&self, id: DispatchID, success: bool) {
        if let Some(tx) = self.lock().await.events.remove(&id) {
            let _ = tx.send(success);
        } else {
            warn!("No dispatcher found for event: {id:?}");
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

    pub async fn wait_for_channel_list(&self) -> Vec<ChannelID> {
        let (tx, rx) = oneshot::channel();
        self.lock().await.channel_list = Some(tx);

        rx.await.expect("Failed to receive channel list")
    }

    pub async fn dispatch_channel_list(&self, channels: Vec<ChannelID>) {
        if let Some(tx) = self.lock().await.channel_list.take() {
            let _ = tx.send(channels);
        } else {
            warn!("No dispatcher found for channel list");
        }
    }

    pub async fn dispatch_channel_info(&self, info: ChannelInfo) {
        if let Some(tx) = self.lock().await.channel_info.take() {
            let _ = tx.send(info);
        } else {
            warn!("No dispatcher found for channel info");
        }
    }

    pub async fn wait_for_channel_info(&self) -> ChannelInfo {
        let (tx, rx) = oneshot::channel();
        self.lock().await.channel_info = Some(tx);

        rx.await.expect("Failed to receive channel info")
    }

    pub async fn register_data_handler(
        &self,
        channel: ChannelID,
        handler: mpsc::Sender<(ChannelID, String)>,
    ) {
        self.lock().await.data_handlers.insert(channel, handler);
    }

    pub async fn dispatch_data(&self, channel: ChannelID, data: String) {
        if let Some(handler) = self.lock().await.data_handlers.get(&channel) {
            let _ = handler.send((channel, data)).await;
        } else {
            warn!("No data handler found for channel: {channel}");
        }
    }

    pub async fn register_bin_data_handler(
        &self,
        channel: ChannelID,
        handler: mpsc::Sender<(ChannelID, Vec<u8>)>,
    ) {
        self.lock().await.bin_data_handlers.insert(channel, handler);
    }

    pub async fn dispatch_bin_data(&self, channel: ChannelID, data: Vec<u8>) {
        if let Some(handler) = self.lock().await.bin_data_handlers.get(&channel) {
            let _ = handler.send((channel, data)).await;
        } else {
            warn!("No binary data handler found for channel: {channel}");
        }
    }

    pub async fn set_node_id(&self, node_id: NodeID) {
        self.lock().await.node_id = Some(node_id);
    }

    pub async fn get_node_id(&self) -> Option<NodeID> {
        self.lock().await.node_id
    }
}

#[derive(Debug)]
pub enum DCClientError {
    WSError(tungstenite::Error),
    ConnectionBroken,
}

impl std::fmt::Display for DCClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DCClientError::WSError(e) => write!(f, "WebSocket error: {e}"),
            DCClientError::ConnectionBroken => write!(f, "Connection broken"),
        }
    }
}

impl std::error::Error for DCClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DCClientError::WSError(e) => Some(e),
            DCClientError::ConnectionBroken => None,
        }
    }
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

    pub async fn listen(
        &mut self,
        channel: ChannelID,
        channel_tx: Option<mpsc::Sender<(ChannelID, String)>>,
        channel_bin_tx: Option<mpsc::Sender<(ChannelID, Vec<u8>)>>,
    ) -> Result<(), DCClientError> {
        if self.listening_channels.contains(&channel) {
            warn!("Channel {channel} is already being listened to");
            return Ok(());
        }
        self.listening_channels.push(channel);

        self.send_evt(Event::ChannelListenRequest { channel })
            .await
            .map_err(DCClientError::WSError)?;

        if let Some(tx) = channel_tx {
            self.dispatches.register_data_handler(channel, tx).await;
        }

        if let Some(bin_tx) = channel_bin_tx {
            self.dispatches
                .register_bin_data_handler(channel, bin_tx)
                .await;
        }

        let response = self
            .dispatches
            .wait_for_event(DispatchID::Listen(channel))
            .await;

        if !response {
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

    pub async fn send_bin(
        &mut self,
        channel: ChannelID,
        data: Vec<u8>,
    ) -> Result<(), DCClientError> {
        self.send_evt(Event::DataBin { channel, data })
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

    pub async fn channel_list(&mut self) -> Result<Vec<ChannelID>, DCClientError> {
        self.send_evt(Event::ChannelListRequest)
            .await
            .map_err(DCClientError::WSError)?;

        let channels = self.dispatches.wait_for_channel_list().await;
        Ok(channels)
    }
    pub async fn channel_info(&mut self, channel: ChannelID) -> Result<ChannelInfo, DCClientError> {
        self.send_evt(Event::ChannelInfoRequest(channel))
            .await
            .map_err(DCClientError::WSError)?;

        let info = self.dispatches.wait_for_channel_info().await;
        Ok(info)
    }

    pub async fn get_node_id(&self) -> Option<NodeID> {
        self.dispatches.get_node_id().await
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
                        info!("Node ID: {node_id}");
                        dispatchers.set_node_id(node_id).await;
                    }
                    Event::Data { channel, data } => {
                        dispatchers.dispatch_data(channel, data).await;
                    }
                    Event::DataBin { channel, data } => {
                        dispatchers.dispatch_bin_data(channel, data).await;
                    }
                    Event::ChannelOpenResponse {
                        channel,
                        success: _,
                    } => {
                        dispatchers.dispatch_channel(channel).await;
                    }
                    Event::ChannelListenResponse { channel, success } => {
                        dispatchers
                            .dispatch_event(DispatchID::Listen(channel), success)
                            .await;
                    }
                    Event::ChannelListResponse { channels } => {
                        dispatchers.dispatch_channel_list(channels).await;
                    }

                    Event::ChannelInfoResponse(info) => {
                        dispatchers.dispatch_channel_info(info).await;
                    }
                    _ => {
                        warn!("Unhandled event: {event:?}");
                    }
                }
            })
            .await;
    }
}
