use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dtos::Todo;

#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub profile_uuid: Uuid,
    pub nickname: String,
    pub full_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct TodoQueue {
    pub queue_uuid: Uuid,
    pub tasks: VecDeque<Todo>,
}

#[derive(Serialize, Deserialize)]
pub struct TaskQueueOverview {
    pub uuid: Uuid,
    pub name: String,
    pub description: String,
}
