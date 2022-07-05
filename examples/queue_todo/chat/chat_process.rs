use std::{collections::HashMap, time::Instant};

use lunatic::{
    process::{AbstractProcess, ProcessRef, ProcessRequest, Request},
    supervisor::Supervisor,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    polling::PollingProcess,
    structures::{Chat, ChatMessage},
};

pub struct ChatSupervisor;
impl Supervisor for ChatSupervisor {
    type Arg = String;
    type Children = ChatProcess;

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
        // Always register the `ChatProcess` under the name passed to the supervisor.
        config.children_args(((), Some(name)))
    }
}

#[derive(Default)]
pub struct ChatProcess {
    /// maps a user_uuid to a vec of chat uuids
    user_chats: HashMap<Uuid, Vec<Uuid>>,
    chats: HashMap<Uuid, Chat>,
    /// maps a user_uuid to a pollingProcess
    user_polls: HashMap<Uuid, ProcessRef<PollingProcess>>,
}

impl ChatProcess {
    pub fn create_chat(&mut self, name: String, participants: Vec<Uuid>) -> &mut Chat {
        let chat_uuid = Uuid::new_v4();
        self.chats.insert(
            chat_uuid,
            Chat {
                chat_uuid,
                name,
                participants,
                messages: vec![],
            },
        );
        self.chats.get_mut(&chat_uuid).unwrap()
    }
}

/// Chat is based on a simple /send action and a /poll action, where
/// /poll stays alive for a longer time to not spam the server with requests
/// and still have an almost "instant messaging"-like experience
impl AbstractProcess for ChatProcess {
    type Arg = ();
    type State = Self;

    fn init(_: ProcessRef<Self>, _: Self::Arg) -> Self::State {
        // Coordinator shouldn't die when a client dies. This makes the link one-directional.
        unsafe { lunatic::host::api::process::die_when_link_dies(0) };
        ChatProcess::default()
    }
}

/// this creates a new chat with a certain name and participants
impl ProcessRequest<(String, Vec<Uuid>)> for ChatProcess {
    type Response = bool;

    fn handle(
        state: &mut Self::State,
        (name, participants): (String, Vec<Uuid>),
    ) -> Self::Response {
        state.create_chat(name, participants);
        true
    }
}

/// this is the /send action
#[derive(Serialize, Deserialize)]
pub struct SendMessage(pub Uuid, pub ChatMessage);
impl ProcessRequest<SendMessage> for ChatProcess {
    type Response = bool;

    fn handle(
        state: &mut Self::State,
        SendMessage(chat_uuid, message): SendMessage,
    ) -> Self::Response {
        let chat = if let Some(c) = state.chats.get_mut(&chat_uuid) {
            c
        } else {
            return false;
        };
        // make sure user is one of participants
        if chat.participants.contains(&message.sender) {
            chat.messages.push(message.clone());
            // go over all active listeners and send a message
            for u in chat.participants.iter() {
                if let Some(listener) = state.user_polls.get(u) {
                    listener.request(vec![message.clone()]);
                }
            }
            return true;
        }
        false
    }
}

#[derive(Serialize, Deserialize)]
pub struct AddListener(pub Uuid, pub ProcessRef<PollingProcess>);
impl ProcessRequest<AddListener> for ChatProcess {
    type Response = bool;

    fn handle(
        state: &mut Self::State,
        AddListener(user_uuid, polling_process): AddListener,
    ) -> Self::Response {
        state.user_polls.insert(user_uuid, polling_process);
        true
    }
}

#[derive(Serialize, Deserialize)]
pub struct DropListener(pub Uuid);
impl ProcessRequest<DropListener> for ChatProcess {
    type Response = bool;

    fn handle(state: &mut Self::State, DropListener(user_uuid): DropListener) -> Self::Response {
        state.user_polls.remove(&user_uuid);
        true
    }
}
