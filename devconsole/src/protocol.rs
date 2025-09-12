use serde::{Deserialize, Serialize};

pub type ChannelID = u64;
pub type NodeID = u64;

pub enum TransactionError {
    ChannelConflicted,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub channel: ChannelID,
    pub name: String,
    pub supplied_by: NodeID,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    NodeIDNotification { node_id: NodeID },

    Data { channel: ChannelID, data: String },
    DataBin { channel: ChannelID, data: Vec<u8> },

    ChannelOpenRequest { name: String },
    ChannelOpenResponse { channel: ChannelID, success: bool },

    ChannelCloseRequest { channel: ChannelID },

    ChannelListenRequest { channel: ChannelID },
    ChannelListenResponse { channel: ChannelID, success: bool },

    ChannelListRequest,
    ChannelListResponse { channels: Vec<ChannelID> },

    ChannelInfoRequest(ChannelID),
    ChannelInfoResponse(ChannelInfo),
}
