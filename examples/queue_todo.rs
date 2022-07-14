use std::{
    collections::{HashMap, VecDeque},
    fs::{DirBuilder, File},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use lunatic::{
    process::{AbstractProcess, ProcessRef, ProcessRequest, Request, StartProcess},
    supervisor::Supervisor,
};
use serde::{Deserialize, Serialize};
use submillisecond::{
    json::Json, params::Params, router, Application, Middleware, NextFn, Response, RouteError,
    Router,
};
use uuid::Uuid;

// =====================================
// Middleware for requests
// =====================================
struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn apply(
        &self,
        req: submillisecond::Request,
        next: impl NextFn,
    ) -> Result<Response, RouteError> {
        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|req_id| req_id.to_str().ok())
            .map(|req_id| req_id.to_string())
            .unwrap_or_else(|| "DEFAULT_REQUEST_ID".to_string());
        println!("[ENTER] request {request_id}");
        let res = next(req);
        println!("[EXIT] request {request_id}");
        res
    }
}

// =====================================
// Persistence utils
// =====================================
const NEWLINE: &[u8] = &[b'\n'];

/// Every Line is a new "state change" entry
/// Each line starts with one of the following keywords
/// that indicate the type of entry
const NEW_USER: u8 = 1;
const PUSH_TODO: u8 = 2;
const POLL_TODO: u8 = 3;

#[derive(Debug)]
pub struct FileLog {
    // cwd: str,
    // file_name: str,
    full_path: PathBuf,
    file: File,
}

#[derive(Serialize, Deserialize)]
struct PushEntry {
    user_uuid: Uuid,
    todo: Todo,
}

impl FileLog {
    pub fn new(cwd: &str, file_name: &str) -> FileLog {
        DirBuilder::new().recursive(true).create(cwd).unwrap();
        let full_path = Path::new(cwd).join(file_name);
        FileLog {
            // cwd,
            // file_name,
            full_path: full_path.clone(),
            file: match File::create(&full_path) {
                Err(why) => panic!("couldn't open {:?}: {}", cwd, why),
                // write 0 as initial cursor
                Ok(file) => file,
            },
        }
    }

    pub fn append_new_user(&mut self, user: &User) {
        self.append(NEW_USER, ron::to_string(user).unwrap().as_bytes())
    }

    pub fn append_poll_todo(&mut self, user_uuid: Uuid, todo_uuid: Uuid) {
        self.append(
            POLL_TODO,
            ron::to_string(&(user_uuid, todo_uuid)).unwrap().as_bytes(),
        )
    }

    pub fn append_push_todo(&mut self, user_uuid: Uuid, todo: Todo) {
        self.append(
            PUSH_TODO,
            ron::to_string(&PushEntry { user_uuid, todo })
                .unwrap()
                .as_bytes(),
        )
    }

    pub fn append(&mut self, header: u8, data: &[u8]) {
        // let x: MyStruct = ron::from_str("(boolean: true, float: 1.23)").unwrap();
        let encoded = base64::encode(data);
        let buf = [&[header], encoded.as_bytes(), NEWLINE].concat();
        match self.file.write_all(&buf) {
            Err(why) => panic!(
                "[FileLog {:?}] couldn't write to file: {}",
                self.full_path, why
            ),
            Ok(_) => println!(
                "[FileLog {:?}] Successfully appended log to file",
                self.full_path
            ),
        };
    }
}

// =====================================
// Persistence process definition
// =====================================
pub struct PersistenceSup;
impl Supervisor for PersistenceSup {
    type Arg = String;
    type Children = PersistenceProcess;

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
        // Always register the `PersistenceProcess` under the name passed to the supervisor.
        config.children_args(((), Some(name)))
    }
}

pub struct PersistenceProcess {
    users: HashMap<Uuid, User>,
    users_nicknames: HashMap<String, Uuid>,
    wal: FileLog,
}

impl AbstractProcess for PersistenceProcess {
    type Arg = ();
    type State = Self;

    fn init(_: ProcessRef<Self>, _: Self::Arg) -> Self::State {
        // Coordinator shouldn't die when a client dies. This makes the link one-directional.
        unsafe { lunatic::host::api::process::die_when_link_dies(0) };
        PersistenceProcess {
            users: HashMap::new(),
            users_nicknames: HashMap::new(),
            wal: FileLog::new("/persistence", "todos.wal"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct AddTodo(Uuid, Todo);
impl ProcessRequest<AddTodo> for PersistenceProcess {
    type Response = bool;

    fn handle(state: &mut Self::State, AddTodo(user_id, todo): AddTodo) -> bool {
        if let Some(user) = state.users.get_mut(&user_id) {
            state.wal.append_push_todo(user.uuid, todo.clone());
            user.todos.push_back(todo);
            return true;
        }
        false
    }
}

impl ProcessRequest<CreateUserDto> for PersistenceProcess {
    type Response = Option<Uuid>;

    fn handle(
        state: &mut Self::State,
        CreateUserDto { nickname, name }: CreateUserDto,
    ) -> Self::Response {
        let user_uuid = Uuid::new_v4();
        if state.users_nicknames.get(&nickname).is_some() {
            // user already exists
            return None;
        }
        let user = User {
            uuid: user_uuid,
            nickname: nickname.clone(),
            name,
            todos: VecDeque::new(),
        };
        state.wal.append_new_user(&user);
        state.users_nicknames.insert(nickname, user_uuid);
        state.users.insert(user_uuid, user);
        Some(user_uuid)
    }
}

#[derive(Serialize, Deserialize)]
struct PollTodo(Uuid);
impl ProcessRequest<PollTodo> for PersistenceProcess {
    type Response = Option<Todo>;

    fn handle(state: &mut Self::State, PollTodo(user_id): PollTodo) -> Self::Response {
        if let Some(user) = state.users.get_mut(&user_id) {
            if let Some(front) = user.todos.front() {
                state.wal.append_poll_todo(user.uuid, front.uuid);
            }
            return user.todos.pop_front();
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
struct PeekTodo(Uuid);
impl ProcessRequest<PeekTodo> for PersistenceProcess {
    // send clone because it will be serialized anyway
    type Response = Option<Todo>;

    fn handle(state: &mut Self::State, PeekTodo(user_id): PeekTodo) -> Self::Response {
        if let Some(user) = state.users.get_mut(&user_id) {
            if let Some(f) = user.todos.front() {
                return Some(f.clone());
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
struct ListTodos(Uuid);
impl ProcessRequest<ListTodos> for PersistenceProcess {
    type Response = Vec<Todo>;

    fn handle(state: &mut Self::State, ListTodos(user_id): ListTodos) -> Self::Response {
        // self.todos_wal
        //     .append_confirmation(message_uuid, pubrel.clone(), SystemTime::now());
        if let Some(user) = state.users.get_mut(&user_id) {
            return user.todos.iter().cloned().collect();
        }
        vec![]
    }
}

// =====================================
// DTOs
// =====================================
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Todo {
    uuid: Uuid,
    title: String,
    description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    uuid: Uuid,
    nickname: String,
    name: String,
    todos: VecDeque<Todo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateUserDto {
    nickname: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateTodoDto {
    title: String,
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateUserResponseDto {
    uuid: Uuid,
}

// routes logic
fn create_user(user: Json<CreateUserDto>) -> Json<CreateUserResponseDto> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup("persistence").unwrap();
    if let Some(uuid) = persistence.request(user.0) {
        return Json(CreateUserResponseDto { uuid });
    }
    panic!("Cannot create user");
}

fn list_todos(params: Params) -> Json<Vec<Todo>> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup("persistence").unwrap();
    let user_id = params.get("user_id").unwrap();
    let todos = persistence.request(ListTodos(Uuid::from_str(user_id).unwrap()));
    submillisecond::json::Json(todos)
}

fn poll_todo(params: Params) -> Json<Todo> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup("persistence").unwrap();
    let user_id = params.get("user_id").unwrap();
    if let Some(todo) = persistence.request(PollTodo(Uuid::from_str(user_id).unwrap())) {
        return submillisecond::json::Json(todo);
    }
    panic!("Cannot poll todo {params:#?}");
}

fn push_todo(params: Params, body: Json<CreateTodoDto>) -> Json<Option<Todo>> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup("persistence").unwrap();
    let user_id = params.get("user_id").unwrap();
    println!("RECEIVED BODY {:?} | {user_id}", body);
    let todo = Todo {
        uuid: Uuid::new_v4(),
        title: body.0.title,
        description: body.0.description,
    };
    if persistence.request(AddTodo(Uuid::from_str(user_id).unwrap(), todo.clone())) {
        return submillisecond::json::Json(Some(todo));
    }
    submillisecond::json::Json(None)
}

fn liveness_check() -> &'static str {
    println!("Running liveness check");
    "{\"status\":\"UP\"}"
}

// has the prefix /api/mgmt
const MGMT_ROUTER: Router = router! {
    GET "/alive" => liveness_check
    GET "/health" => liveness_check
    GET "/metrics" => liveness_check
};

const ROUTER: Router = router! {
    use LoggingMiddleware;

    "/api/users" => {
        POST "/" => create_user
        "/:user_id" => {
            GET "/todos"  => list_todos
            POST "/todos" => push_todo
            POST "/todos/poll" => poll_todo
        }
    }
    "/api/mgmt" => MGMT_ROUTER
    GET "/something_different/:shoppingcart_id" => liveness_check
};

fn main() -> io::Result<()> {
    PersistenceSup::start_link("persistence".to_owned(), None);

    Application::new(ROUTER).serve("0.0.0.0:3000")
}
