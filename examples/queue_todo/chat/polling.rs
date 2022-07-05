use lunatic::{
    process::{AbstractProcess, ProcessRef, ProcessRequest, Request},
    sleep,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::{
    chat_process::{AddListener, ChatProcess, DropListener},
    structures::ChatMessage,
};

pub struct PollingProcess {
    this: ProcessRef<PollingProcess>,
    received_messages: Option<Vec<ChatMessage>>,
    chat: ProcessRef<ChatProcess>,
    user_uuid: Uuid,
}

impl AbstractProcess for PollingProcess {
    type Arg = Uuid;
    type State = Self;

    fn init(this: ProcessRef<Self>, user_uuid: Self::Arg) -> Self::State {
        let chat = ProcessRef::<ChatProcess>::lookup("chat").unwrap();
        chat.request(AddListener(user_uuid, this.clone()));
        PollingProcess {
            this,
            received_messages: None,
            user_uuid,
            chat,
        }
    }

    fn terminate(state: Self::State) {
        state.chat.request(DropListener(state.user_uuid));
    }
}

#[derive(Serialize, Deserialize)]
/// this polls for messages and handles the polling in a way
/// that doesn't block the chat process
pub struct PollRequest(
    // user_uuid
    Uuid,
);

impl ProcessRequest<PollRequest> for PollingProcess {
    type Response = Vec<ChatMessage>;

    fn handle(state: &mut Self::State, PollRequest(user_uuid): PollRequest) -> Self::Response {
        loop {
            if let Some(msg) = state.received_messages.as_ref() {
                return msg.to_owned();
            }
            sleep(Duration::from_secs(1));
        }
    }
}

impl ProcessRequest<Vec<ChatMessage>> for PollingProcess {
    type Response = bool;

    fn handle(state: &mut Self::State, messages: Vec<ChatMessage>) -> Self::Response {
        state.received_messages = Some(messages);
        true
    }
}
