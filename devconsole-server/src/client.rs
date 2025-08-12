use devconsole_protocol::{ChannelID, Event, NodeID};
use futures_util::SinkExt;
use futures_util::lock::Mutex;
use futures_util::stream::SplitSink;
use log::{error, info};
use std::cell::RefCell;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

struct Client {
    writer: SplitSink<WebSocketStream<TcpStream>, Message>,
    node_id: NodeID,
    listening_channels: RefCell<Vec<ChannelID>>,
}

#[derive(Clone)]
pub struct SharedClient(Arc<Mutex<Client>>);

impl PartialEq for SharedClient {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl SharedClient {
    pub fn new(writer: SplitSink<WebSocketStream<TcpStream>, Message>, node_id: NodeID) -> Self {
        SharedClient(Arc::new(Mutex::new(Client {
            writer,
            node_id,
            listening_channels: RefCell::new(Vec::new()),
        })))
    }

    pub async fn node_id(&self) -> NodeID {
        self.0.lock().await.node_id
    }

    pub async fn send_event(&self, event: Event) -> Result<(), String> {
        // info!("Sending event: {:?}", event);
        let msg = serde_json::to_string(&event).unwrap().into();
        self.0
            .lock()
            .await
            .writer
            .send(Message::Text(msg))
            .await
            .map_err(|e| {
                error!("Error sending event: {}", e);
                e.to_string()
            })?;

        Ok(())
    }

    pub async fn is_listening(&self, channel: ChannelID) -> bool {
        let client = self.0.lock().await;
        client.listening_channels.borrow().contains(&channel)
    }

    pub async fn listen(&self, channel: ChannelID) -> Result<(), String> {
        let client = self.0.lock().await;
        if !client.listening_channels.borrow().contains(&channel) {
            client.listening_channels.borrow_mut().push(channel);
            Ok(())
        } else {
            Err(format!("Already listening to channel {}", channel))
        }
    }
}
