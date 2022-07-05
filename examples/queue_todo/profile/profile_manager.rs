use std::collections::{HashMap, VecDeque};

use lunatic::{
    process::{AbstractProcess, ProcessRef, ProcessRequest},
    supervisor::Supervisor,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    dtos::{CreateUserDto, CreateUserQueueDto, TaskQueue, Todo, User},
    file_log::FileLog,
};

use super::structure::{TaskQueueOverview, UserProfile};

// =====================================
// Profile process definition
// =====================================
pub struct ProfileSup;
impl Supervisor for ProfileSup {
    type Arg = String;
    type Children = ProfileProcess;

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
        // Always register the `ProfileProcess` under the name passed to the supervisor.
        config.children_args(((), Some(name)))
    }
}

#[derive(Default)]
pub struct ProfileProcess {
    // mapping of user_uuid to UserProfile
    user_profiles: HashMap<Uuid, UserProfile>,
    user_queues: HashMap<Uuid, Vec<Uuid>>,
    // mapping of queue_uuid to queue
    queues: HashMap<Uuid, TaskQueue>,
}

impl AbstractProcess for ProfileProcess {
    type Arg = ();
    type State = Self;

    fn init(_: ProcessRef<Self>, _: Self::Arg) -> Self::State {
        // Coordinator shouldn't die when a client dies. This makes the link one-directional.
        unsafe { lunatic::host::api::process::die_when_link_dies(0) };
        ProfileProcess::default()
    }
}

#[derive(Serialize, Deserialize)]
pub struct AddTodo(
    /// user_uuid
    pub Uuid,
    /// queue_uuid
    pub Uuid,
    pub Todo,
);
impl ProcessRequest<AddTodo> for ProfileProcess {
    type Response = bool;

    fn handle(state: &mut Self::State, AddTodo(user_uuid, queue_uuid, todo): AddTodo) -> bool {
        if let Some(queue) = state.queues.get_mut(&queue_uuid) {
            // ensure it's the correct user
            if queue.user_uuid == user_uuid {
                queue.todos.push_back(todo);
                return true;
            }
        }
        false
    }
}

/// returns uuid of newly created
impl ProcessRequest<CreateUserQueueDto> for ProfileProcess {
    type Response = Uuid;

    fn handle(
        state: &mut Self::State,
        CreateUserQueueDto {
            user_uuid,
            queue_name,
            description,
        }: CreateUserQueueDto,
    ) -> Self::Response {
        let queue_uuid = Uuid::new_v4();

        state.queues.insert(
            queue_uuid,
            TaskQueue {
                uuid: queue_uuid,
                user_uuid,
                name: queue_name,
                description,
                todos: VecDeque::new(),
            },
        );
        let user_queues = state.user_queues.entry(user_uuid).or_insert(vec![]);
        user_queues.push(queue_uuid);
        queue_uuid
    }
}

#[derive(Serialize, Deserialize)]
pub struct PollTodo(pub Uuid, pub Uuid);
impl ProcessRequest<PollTodo> for ProfileProcess {
    type Response = Option<Todo>;

    fn handle(
        state: &mut Self::State,
        PollTodo(user_uuid, queue_uuid): PollTodo,
    ) -> Self::Response {
        if let Some(queue) = state.queues.get_mut(&queue_uuid) {
            // ensure it's the correct user
            if queue.user_uuid == user_uuid {
                return queue.todos.pop_front();
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
pub struct PeekTodo(pub Uuid, pub Uuid);
impl ProcessRequest<PeekTodo> for ProfileProcess {
    // send clone because it will be serialized anyway
    type Response = Option<Todo>;

    fn handle(
        state: &mut Self::State,
        PeekTodo(user_uuid, queue_uuid): PeekTodo,
    ) -> Self::Response {
        if let Some(queue) = state.queues.get_mut(&queue_uuid) {
            if let Some(f) = queue.todos.front() {
                return Some(f.clone());
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListTodos(pub Uuid, pub Uuid);
impl ProcessRequest<ListTodos> for ProfileProcess {
    type Response = Vec<Todo>;

    fn handle(
        state: &mut Self::State,
        ListTodos(user_uuid, queue_uuid): ListTodos,
    ) -> Self::Response {
        // self.todos_wal
        //     .append_confirmation(message_uuid, pubrel.clone(), SystemTime::now());
        if let Some(queue) = state.queues.get_mut(&queue_uuid) {
            if queue.user_uuid == user_uuid {
                return queue.todos.iter().map(|t| t.clone()).collect();
            }
        }
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListQueues(pub Uuid);
impl ProcessRequest<ListQueues> for ProfileProcess {
    type Response = Vec<TaskQueueOverview>;

    fn handle(state: &mut Self::State, ListQueues(user_uuid): ListQueues) -> Self::Response {
        if let Some(queue_ids) = state.user_queues.get_mut(&user_uuid) {
            return queue_ids
                .iter()
                .map(|id| {
                    let q = state.queues.get(id).unwrap();
                    TaskQueueOverview {
                        uuid: q.uuid,
                        name: q.name.clone(),
                        description: q.description.clone(),
                    }
                })
                .collect();
        }
        vec![]
    }
}
