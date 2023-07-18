use std::collections::{HashMap, VecDeque};
use std::fs::{DirBuilder, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use base64::engine::general_purpose;
use base64::Engine as _;
use lunatic::ap::{AbstractProcess, Config, ProcessRef};
use lunatic::supervisor::{Supervisor, SupervisorConfig};
use lunatic::{abstract_process, ProcessName};
use serde::{Deserialize, Serialize};
use submillisecond::params::Params;
use submillisecond::response::Response;
use submillisecond::{router, Application, Json, RequestContext, Router};
use uuid::Uuid;

// =====================================
// Middleware for requests
// =====================================
fn logging_middleware(req: RequestContext) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|req_id| req_id.to_str().ok())
        .map(|req_id| req_id.to_string())
        .unwrap_or_else(|| "DEFAULT_REQUEST_ID".to_string());
    println!("[ENTER] request {request_id}");
    let res = req.next_handler();
    println!("[EXIT] request {request_id}");
    res
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
                Err(why) => panic!("couldn't open {cwd:?}: {why}"),
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
        let encoded = general_purpose::STANDARD.encode(data);
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
    type Children = (PersistenceProcess,);

    fn init(config: &mut SupervisorConfig<Self>, name: Self::Arg) {
        // Always register the `PersistenceProcess` under the name passed to the
        // supervisor.
        config.children_args((((), Some(name)),))
    }
}

#[derive(ProcessName)]
pub struct PersistenceProcessID;

pub struct PersistenceProcess {
    users: HashMap<Uuid, User>,
    users_nicknames: HashMap<String, Uuid>,
    wal: FileLog,
}

#[abstract_process(visibility=pub)]
impl PersistenceProcess {
    #[init]
    fn init(_: Config<Self>, _: ()) -> Result<PersistenceProcess, ()> {
        // Coordinator shouldn't die when a client dies. This makes the link
        // one-directional.
        unsafe { lunatic::host::api::process::die_when_link_dies(0) };
        Ok(PersistenceProcess {
            users: HashMap::new(),
            users_nicknames: HashMap::new(),
            wal: FileLog::new("/persistence", "todos.wal"),
        })
    }

    #[handle_request]
    fn add_todo(&mut self, user_id: Uuid, todo: Todo) -> bool {
        if let Some(user) = self.users.get_mut(&user_id) {
            self.wal.append_push_todo(user.uuid, todo.clone());
            user.todos.push_back(todo);
            return true;
        }
        false
    }

    #[handle_request]
    fn create_user(&mut self, CreateUserDto { nickname, name }: CreateUserDto) -> Option<Uuid> {
        let user_uuid = Uuid::new_v4();
        if self.users_nicknames.get(&nickname).is_some() {
            // user already exists
            return None;
        }
        let user = User {
            uuid: user_uuid,
            nickname: nickname.clone(),
            name,
            todos: VecDeque::new(),
        };
        self.wal.append_new_user(&user);
        self.users_nicknames.insert(nickname, user_uuid);
        self.users.insert(user_uuid, user);
        Some(user_uuid)
    }

    #[handle_request]
    fn poll_todo(&mut self, user_id: Uuid) -> Option<Todo> {
        if let Some(user) = self.users.get_mut(&user_id) {
            if let Some(front) = user.todos.front() {
                self.wal.append_poll_todo(user.uuid, front.uuid);
            }
            return user.todos.pop_front();
        }
        None
    }

    #[handle_request]
    fn peek_todo(&mut self, user_id: Uuid) -> Option<Todo> {
        if let Some(user) = self.users.get_mut(&user_id) {
            if let Some(f) = user.todos.front() {
                return Some(f.clone());
            }
        }
        None
    }

    #[handle_request]
    fn list_todos(&mut self, user_id: Uuid) -> Vec<Todo> {
        // self.todos_wal
        //     .append_confirmation(message_uuid, pubrel.clone(), SystemTime::now());
        if let Some(user) = self.users.get_mut(&user_id) {
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
pub struct CreateUserDto {
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
    let persistence = ProcessRef::<PersistenceProcess>::lookup(&"persistence").unwrap();
    if let Some(uuid) = persistence.create_user(user.0) {
        return Json(CreateUserResponseDto { uuid });
    }
    panic!("Cannot create user");
}

fn list_todos(params: Params) -> Json<Vec<Todo>> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup(&PersistenceProcessID).unwrap();
    let user_id = params.get("user_id").unwrap();
    let todos = persistence.list_todos(Uuid::from_str(user_id).unwrap());
    Json(todos)
}

fn poll_todo(params: Params) -> Json<Todo> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup(&PersistenceProcessID).unwrap();
    let user_id = params.get("user_id").unwrap();
    if let Some(todo) = persistence.poll_todo(Uuid::from_str(user_id).unwrap()) {
        return Json(todo);
    }
    panic!("Cannot poll todo {params:#?}");
}

fn push_todo(params: Params, body: Json<CreateTodoDto>) -> Json<Option<Todo>> {
    let persistence = ProcessRef::<PersistenceProcess>::lookup(&PersistenceProcessID).unwrap();
    let user_id = params.get("user_id").unwrap();
    println!("RECEIVED BODY {body:?} | {user_id}");
    let todo = Todo {
        uuid: Uuid::new_v4(),
        title: body.0.title,
        description: body.0.description,
    };
    if persistence.add_todo(Uuid::from_str(user_id).unwrap(), todo.clone()) {
        return Json(Some(todo));
    }
    Json(None)
}

fn liveness_check() -> &'static str {
    println!("Running liveness check");
    r#"{"status":"UP"}"#
}

// has the prefix /api/mgmt
const MGMT_ROUTER: Router = router! {
    GET "/alive" => liveness_check
    GET "/health" => liveness_check
    GET "/metrics" => liveness_check
};

const ROUTER: Router = router! {
    with logging_middleware;

    "/api/users" => {
        POST "/" => create_user
        "/:user_id" => {
            GET "/todos"  => list_todos
            POST "/todos" => push_todo
            POST "/todos/poll" => poll_todo
        }
    }
    "/api/mgmt" => MGMT_ROUTER()
    GET "/something_different/:shoppingcart_id" => liveness_check
};

fn main() -> io::Result<()> {
    PersistenceSup::link()
        .start("persistence".to_owned())
        .unwrap();

    Application::new(ROUTER).serve("0.0.0.0:3000")
}
