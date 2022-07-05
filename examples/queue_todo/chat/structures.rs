use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    #[serde(with = "serde_millis")]
    pub sent_at: Instant,
    pub sender: Uuid,
    pub mentions: Vec<Uuid>,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Chat {
    pub chat_uuid: Uuid,
    pub name: String,
    pub participants: Vec<Uuid>,
    pub messages: Vec<ChatMessage>,
}
