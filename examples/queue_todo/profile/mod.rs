use std::str::FromStr;

use crate::{
    dtos::{
        CreateTodoDto, CreateUserDto, CreateUserQueueDto, CreateUserResponseDto,
        CreatedQueueResponseDto, Todo, TodoResponseDto,
    },
    profile::profile_manager::{ListQueues, ProfileProcess},
};
use lunatic::process::{ProcessRef, Request};
use serde::{Deserialize, Serialize};
use submillisecond::{
    extract::{FromRequest, Path},
    handler::HandlerFn,
    json::Json,
    params::Params,
    router,
};
use uuid::Uuid;

use self::{
    profile_manager::{AddTodo, ListTodos, PollTodo},
    structure::TaskQueueOverview,
};

pub mod profile_manager;
pub mod structure;

// routes logic
fn create_queue(
    Path(user_uuid): Path<Uuid>,
    queue: Json<CreateUserQueueDto>,
) -> Json<CreatedQueueResponseDto> {
    let profile = ProcessRef::<ProfileProcess>::lookup("profile").unwrap();
    let queue_uuid = profile.request(queue.0);
    return Json(CreatedQueueResponseDto {
        queue_uuid,
        user_uuid,
    });
}

fn list_queues(Path(user_uuid): Path<Uuid>) -> Json<Vec<TaskQueueOverview>> {
    let profile = ProcessRef::<ProfileProcess>::lookup("profile").unwrap();
    let queue_list = profile.request(ListQueues(user_uuid));
    return Json(queue_list);
}

fn list_todos(Path((user_id, queue_id)): Path<(Uuid, Uuid)>) -> Json<Vec<Todo>> {
    let profile = ProcessRef::<ProfileProcess>::lookup("profile").unwrap();
    let todos = profile.request(ListTodos(user_id, queue_id));
    submillisecond::json::Json(todos)
}

fn poll_todo(Path((user_id, queue_id)): Path<(Uuid, Uuid)>) -> Json<TodoResponseDto> {
    let profile = ProcessRef::<ProfileProcess>::lookup("profile").unwrap();
    if let Some(todo) = profile.request(PollTodo(user_id, queue_id)) {
        return submillisecond::json::Json(TodoResponseDto { todo: Some(todo) });
    }
    Json(TodoResponseDto::default())
}

fn push_todo(
    Path((user_id, queue_id)): Path<(Uuid, Uuid)>,
    body: Json<CreateTodoDto>,
) -> Json<Option<Todo>> {
    let profile = ProcessRef::<ProfileProcess>::lookup("profile").unwrap();
    let todo = Todo {
        uuid: Uuid::new_v4(),
        title: body.0.title,
        description: body.0.description,
    };
    if profile.request(AddTodo(user_id, queue_id, todo.clone())) {
        return submillisecond::json::Json(Some(todo));
    }
    return submillisecond::json::Json(None);
}

/// handles /profile routes
pub static PROFILE_ROUTER: HandlerFn = router! {
    "/:user_id/queues/:queue_id" => {
        POST "/" => create_queue
        GET "/" => list_queues
        GET "/todos"  => list_todos
        POST "/todos" => push_todo
        POST "/todos/poll" => poll_todo
    }
};
