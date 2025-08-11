use serde::{Deserialize, Serialize};

pub type ChannelID = u64;
pub type NodeID = u64;

pub enum TransactionError {
    ChannelConflicted,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    NodeIDNotification {
        node_id: NodeID,
    },

    Data {
        channel: ChannelID,
        data: String,
    },

    ChannelOpenRequest {
        name: String,
    },
    ChannelOpenResponse {
        channel: ChannelID,
        success: bool,
    },

    ChannelCloseRequest {
        channel: ChannelID,
    },

    ChannelListenRequest {
        channel: ChannelID,
    },
    ChannelListenResponse {
        channel: ChannelID,
        success: bool,
    },

    ChannelListRequest,
    ChannelListResponse {
        channels: Vec<ChannelID>,
    },

    ChannelInfoRequest(ChannelID),
    ChannelInfoResponse {
        channel: ChannelID,
        name: String,
        supplied_by: NodeID,
    },
}
