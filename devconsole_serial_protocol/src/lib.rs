use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum SerialEvent {
    Opened { path: String },
    Line { path: String, line: Vec<u8> },
    Closed { path: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SerialRequest {
    Data { path: String, data: Vec<u8> },
}
