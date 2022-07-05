// use std::collections::HashMap;

// use lunatic::{
//     process::{AbstractProcess, ProcessRef, ProcessRequest},
//     supervisor::Supervisor,
// };
// use serde::{Deserialize, Serialize};
// use uuid::Uuid;

// use crate::{
//     dtos::{CreateUserDto, Todo, User},
//     file_log::FileLog,
// };

// // =====================================
// // Persistence process definition
// // =====================================
// pub struct PersistenceSup;
// impl Supervisor for PersistenceSup {
//     type Arg = String;
//     type Children = PersistenceProcess;

//     fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
//         // Always register the `PersistenceProcess` under the name passed to the supervisor.
//         config.children_args(((), Some(name)))
//     }
// }

// pub struct PersistenceProcess {
//     users: HashMap<Uuid, User>,
//     users_nicknames: HashMap<String, Uuid>,
//     wal: FileLog,
// }

// impl AbstractProcess for PersistenceProcess {
//     type Arg = ();
//     type State = Self;

//     fn init(_: ProcessRef<Self>, _: Self::Arg) -> Self::State {
//         // Coordinator shouldn't die when a client dies. This makes the link one-directional.
//         unsafe { lunatic::host::api::process::die_when_link_dies(0) };
//         PersistenceProcess {
//             users: HashMap::new(),
//             users_nicknames: HashMap::new(),
//             wal: FileLog::new("/persistence", "todos.wal"),
//         }
//     }
// }

// #[derive(Serialize, Deserialize)]
// struct AddTodo(Uuid, Todo);
// impl ProcessRequest<AddTodo> for PersistenceProcess {
//     type Response = bool;

//     fn handle(state: &mut Self::State, AddTodo(user_id, todo): AddTodo) -> bool {
//         if let Some(user) = state.users.get_mut(&user_id) {
//             // state.wal.append_push_todo(user.uuid, todo.clone());
//             user.todos.push_back(todo);
//             return true;
//         }
//         false
//     }
// }

// impl ProcessRequest<CreateUserDto> for PersistenceProcess {
//     type Response = Option<Uuid>;

//     fn handle(
//         state: &mut Self::State,
//         CreateUserDto { nickname, name }: CreateUserDto,
//     ) -> Self::Response {
//         let user_uuid = Uuid::new_v4();
//         if let Some(_) = state.users_nicknames.get(&nickname) {
//             // user already exists
//             return None;
//         }
//         let user = User {
//             uuid: user_uuid,
//             nickname: nickname.clone(),
//             full_name: name,
//         };
//         // state.wal.append_new_user(&user);
//         state.users_nicknames.insert(nickname, user_uuid);
//         state.users.insert(user_uuid, user);
//         Some(user_uuid)
//     }
// }

// #[derive(Serialize, Deserialize)]
// struct PollTodo(Uuid);
// impl ProcessRequest<PollTodo> for PersistenceProcess {
//     type Response = Option<Todo>;

//     fn handle(state: &mut Self::State, PollTodo(user_id): PollTodo) -> Self::Response {
//         if let Some(user) = state.users.get_mut(&user_id) {
//             if let Some(front) = user.todos.front() {
//                 // state.wal.append_poll_todo(user.uuid, front.uuid);
//             }
//             return user.todos.pop_front();
//         }
//         None
//     }
// }

// #[derive(Serialize, Deserialize)]
// struct PeekTodo(Uuid);
// impl ProcessRequest<PeekTodo> for PersistenceProcess {
//     // send clone because it will be serialized anyway
//     type Response = Option<Todo>;

//     fn handle(state: &mut Self::State, PeekTodo(user_id): PeekTodo) -> Self::Response {
//         if let Some(user) = state.users.get_mut(&user_id) {
//             if let Some(f) = user.todos.front() {
//                 return Some(f.clone());
//             }
//         }
//         None
//     }
// }

// #[derive(Serialize, Deserialize)]
// struct ListTodos(Uuid);
// impl ProcessRequest<ListTodos> for PersistenceProcess {
//     type Response = Vec<Todo>;

//     fn handle(state: &mut Self::State, ListTodos(user_id): ListTodos) -> Self::Response {
//         // self.todos_wal
//         //     .append_confirmation(message_uuid, pubrel.clone(), SystemTime::now());
//         if let Some(user) = state.users.get_mut(&user_id) {
//             return user.todos.iter().map(|t| t.clone()).collect();
//         }
//         vec![]
//     }
// }
