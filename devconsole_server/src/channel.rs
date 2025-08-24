use devconsole::{ChannelID, NodeID};

#[derive(Clone)]
pub struct Channel {
    id: ChannelID,
    name: String,
    supplied_by: NodeID,
}

impl Channel {
    pub fn new(id: ChannelID, name: String, supplied_by: NodeID) -> Self {
        Channel {
            id,
            name,
            supplied_by,
        }
    }

    pub fn id(&self) -> ChannelID {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn supplied_by(&self) -> NodeID {
        self.supplied_by
    }
}
