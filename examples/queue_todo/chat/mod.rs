use lunatic::process::ProcessRef;
use serde::{Deserialize, Serialize};
use submillisecond::{
    extract::{Path, TypedHeader},
    handler::HandlerFn,
    json::Json,
    params::Param,
    router,
};

use self::{chat_process::ChatProcess, structures::ChatMessage};

pub mod chat_process;
mod polling;
mod structures;

#[derive(Serialize, Deserialize)]
pub struct MessageResponse {
    success: bool,
}

pub fn send_message(
    Path(chat_id): Path<String>,
    Json(message): Json<ChatMessage>,
) -> Json<MessageResponse> {
    let chat = ProcessRef::<ChatProcess>::lookup("chat").unwrap();
    Json(MessageResponse { success: true })
}
pub fn poll_message(Path(user_id): Path<String>) -> Json<MessageResponse> {
    Json(MessageResponse { success: true })
}
pub fn init_chat(Path(user_id): Path<String>) -> Json<MessageResponse> {
    Json(MessageResponse { success: true })
}

pub static CHAT_ROUTER: HandlerFn = router! {
    "/:chat_id" => {
        POST "/send" => send_message
        POST "/poll" => poll_message
        POST "/init" => init_chat
    }
};
