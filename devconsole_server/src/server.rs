use devconsole::{ChannelID, Event, NodeID};
use futures_util::lock::Mutex;
use std::sync::Arc;

use crate::{channel::Channel, client::SharedClient, id_manager::IDManager};

struct Server {
    node_id_manager: IDManager<NodeID>,
    channel_id_manager: IDManager<ChannelID>,
    channels: Vec<Channel>,

    connections: Vec<SharedClient>,
}

#[derive(Clone)]
pub struct SharedServer(Arc<Mutex<Server>>);
impl SharedServer {
    pub fn new_default() -> Self {
        SharedServer(Arc::new(Mutex::new(Server {
            node_id_manager: IDManager::new(),
            channel_id_manager: IDManager::new(),
            channels: Vec::new(),
            connections: Vec::new(),
        })))
    }

    pub async fn get_new_node_id(&self) -> NodeID {
        self.0.lock().await.node_id_manager.get_new_id()
    }

    pub async fn new_channel(&self, name: String, supplied_by: NodeID) -> ChannelID {
        let cid = self.0.lock().await.channel_id_manager.get_new_id();

        let channel = Channel::new(cid, name, supplied_by);
        self.0.lock().await.channels.push(channel);

        cid
    }

    pub async fn broadcast_data(&self, channel: ChannelID, data: String, from: NodeID) {
        for client in &self.0.lock().await.connections {
            if client.node_id().await == from {
                continue;
            }
            if client.is_listening(channel).await {
                let event = Event::Data {
                    channel,
                    data: data.clone(),
                };
                client.send_event(event).await.unwrap();
            }
        }
    }

    pub async fn broadcast_bin_data(&self, channel: ChannelID, data: Vec<u8>, from: NodeID) {
        for client in &self.0.lock().await.connections {
            if client.node_id().await == from {
                continue;
            }
            if client.is_listening(channel).await {
                let event = Event::DataBin {
                    channel,
                    data: data.clone(),
                };
                client.send_event(event).await.unwrap();
            }
        }
    }
    pub async fn add_connection(&self, client: SharedClient) {
        self.0.lock().await.connections.push(client);
    }

    pub async fn remove_connection(&self, client: &SharedClient) {
        let client_node_id = client.node_id().await;

        self.0.lock().await.connections.retain(|c| c != client);

        // remove channels that are provided by this client
        self.0
            .lock()
            .await
            .channels
            .retain(|c| c.supplied_by() != client_node_id);
    }

    pub async fn get_channel_ids(&self) -> Vec<ChannelID> {
        self.0
            .lock()
            .await
            .channels
            .iter()
            .map(|c| c.id())
            .collect()
    }

    pub async fn get_channel(&self, channel_id: ChannelID) -> Option<Channel> {
        self.0
            .lock()
            .await
            .channels
            .iter()
            .find(|c| c.id() == channel_id)
            .cloned()
    }
}
