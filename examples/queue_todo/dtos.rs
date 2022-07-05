use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =====================================
// DTOs
// =====================================
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Todo {
    pub(crate) uuid: Uuid,
    pub(crate) title: String,
    pub(crate) description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub(crate) uuid: Uuid,
    pub(crate) nickname: String,
    pub(crate) full_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct TaskQueue {
    pub(crate) uuid: Uuid,
    pub(crate) user_uuid: Uuid,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) todos: VecDeque<Todo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserDto {
    pub nickname: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserQueueDto {
    pub user_uuid: Uuid,
    pub queue_name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTodoDto {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserResponseDto {
    pub uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreatedQueueResponseDto {
    pub user_uuid: Uuid,
    pub queue_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TodoResponseDto {
    pub todo: Option<Todo>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreateChatDto {
    pub participants: Vec<Uuid>,
    pub name: String,
}
