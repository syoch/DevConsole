type Channel = u64;

pub enum Event {
    Accepted,
    Denied,
    Conflicted,

    Data { channel: Channel, data: String },

    ChannelOpen { name: String },
    ChannelOpenResult { channel: Channel, success: bool },

    ChannelClosed { channel: Channel },
    ChannelListen { channel: Channel },

    ChannelListRequest,
    ChannelListResponse { channels: Vec<Channel> },

    ChannelInfoRequst,
}
