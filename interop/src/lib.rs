use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    ChatMessage { name: String, message: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {}
