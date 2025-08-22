use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum SerialEvent {
    Opened { path: String },
    Line { path: String, line: String },
    Closed { path: String },
}
