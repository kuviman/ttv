use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    ChatMessage { name: String, message: String },
    RewardRedemption { name: String, reward: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {}
