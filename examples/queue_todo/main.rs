use std::{
    collections::{HashMap, VecDeque},
    io,
    str::FromStr,
};

use auth::{AuthSup, AUTH_ROUTER};
use chat::CHAT_ROUTER;
use dtos::{CreateTodoDto, CreateUserDto, CreateUserResponseDto, Todo, User};
use file_log::FileLog;
use lunatic::{
    process::{AbstractProcess, ProcessRef, ProcessRequest, Request, StartProcess},
    supervisor::Supervisor,
};
use profile::{PROFILE_ROUTER, profile_manager::ProfileSup};
use serde::{Deserialize, Serialize};
use submillisecond::{handler::HandlerFn, json::Json, params::Params, router, Application};
use uuid::Uuid;

mod auth;
mod chat;
mod dtos;
mod file_log;
mod middleware;
mod persistence;
mod profile;

use chat::chat_process::ChatSupervisor;

// use persistence::PersistenceSup;

fn liveness_check() -> &'static str {
    println!("Running liveness check");
    "{\"status\":\"UP\"}"
}

// has the prefix /api/mgmt
const MGMT_ROUTER: HandlerFn = router! {
    GET "/alive" => liveness_check
    GET "/health" => liveness_check
    GET "/metrics" => liveness_check
};

const ROUTER: HandlerFn = router! {
    use middleware::LoggingMiddleware;

    "/api/auth" => AUTH_ROUTER
    "/api/users" => PROFILE_ROUTER
    "/api/mgmt" => MGMT_ROUTER
    "/api/chat" => CHAT_ROUTER
};

fn main() -> io::Result<()> {
    // PersistenceSup::start_link("persistence".to_owned(), None);
    AuthSup::start_link("auth_manager".to_string(), None);
    ChatSupervisor::start_link("chat".to_string(), None);
    ProfileSup::start_link("profile".to_string(), None);

    Application::new(ROUTER).serve("0.0.0.0:3000")
}
