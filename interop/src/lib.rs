use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    ChatMessage {
        name: String,
        message: String,
    },
    RewardRedemption {
        name: String,
        reward: String,
    },
    KeyValue {
        request_id: String,
        value: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {
    Say { text: String },
    GetKeyValue { request_id: String, key: String },
    SetKeyValue { key: String, value: String },
}
