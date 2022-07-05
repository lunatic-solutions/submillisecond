use std::{collections::HashMap, time::Instant};

use lunatic::{
    process::{AbstractProcess, ProcessRef, ProcessRequest},
    supervisor::Supervisor,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::session::{hash_password, is_same_password, Session};
use super::structures::{LoginDto, Signup};

use crate::profile::structure::UserProfile;

pub struct AuthSup;
impl Supervisor for AuthSup {
    type Arg = String;
    type Children = AuthProcess;

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
        // Always register the `AuthProcess` under the name passed to the supervisor.
        config.children_args(((), Some(name)))
    }
}

struct AuthUser {
    user_uuid: Uuid,
    profile_uuid: Uuid,
    password_hash: String,
}

pub struct AuthProcess {
    users: HashMap<Uuid, AuthUser>,
    users_nicknames: HashMap<String, Uuid>,
    sessions: HashMap<Uuid, Session>, // wal: FileLog,
}

impl AbstractProcess for AuthProcess {
    type Arg = ();
    type State = Self;

    fn init(_: ProcessRef<Self>, _: Self::Arg) -> Self::State {
        // Coordinator shouldn't die when a client dies. This makes the link one-directional.
        unsafe { lunatic::host::api::process::die_when_link_dies(0) };
        AuthProcess {
            users: HashMap::new(),
            users_nicknames: HashMap::new(),
            sessions: HashMap::new(), // wal: FileLog::new("/persistence", "todos.wal"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum SignupFailureReason {
    UserAlreadyExists,
    InvalidEmail,
    InvalidPassword,
}

#[derive(Serialize, Deserialize)]
pub enum SignupUserResponse {
    Success(Session),
    Failed(SignupFailureReason),
}
impl ProcessRequest<Signup> for AuthProcess {
    type Response = SignupUserResponse;

    fn handle(state: &mut Self::State, signup: Signup) -> SignupUserResponse {
        let user_uuid = Uuid::new_v4();
        if let Some(_) = state.users_nicknames.get(&signup.nickname) {
            // user already exists
            return SignupUserResponse::Failed(SignupFailureReason::UserAlreadyExists);
        }
        let user_profile = UserProfile {
            profile_uuid: Uuid::new_v4(),
            nickname: signup.nickname.clone(),
            full_name: signup.name,
        };
        let user = AuthUser {
            user_uuid,
            profile_uuid: user_profile.profile_uuid,
            password_hash: hash_password(signup.password),
        };
        // state.wal.append_new_user(&user);
        state.users_nicknames.insert(signup.nickname, user_uuid);
        state.users.insert(user_uuid, user);
        // log in user
        let session = Session::new(user_uuid.clone());
        state.sessions.insert(session.uuid, session.clone());
        SignupUserResponse::Success(session)
    }
}

#[derive(Serialize, Deserialize)]
/// Wraps Session UUID
pub enum AuthResponse {
    LoggedIn(Session),
    UserNotFound,
    InvalidPassword,
}
impl ProcessRequest<LoginDto> for AuthProcess {
    type Response = AuthResponse;

    fn handle(
        state: &mut Self::State,
        LoginDto { nickname, password }: LoginDto,
    ) -> Self::Response {
        if let Some(user_uuid) = state.users_nicknames.get(&nickname) {
            let user = state.users.get(user_uuid).expect("user should exist");
            if is_same_password(password, &user.password_hash) {
                let session = Session::new(user_uuid.clone());
                state.sessions.insert(session.uuid, session.clone());
                return AuthResponse::LoggedIn(session);
            }
            return AuthResponse::InvalidPassword;
        }
        // user doesn't exist
        return AuthResponse::UserNotFound;
    }
}

#[derive(Serialize, Deserialize)]
pub struct IsAuthorized(
    /// session uuid
    Option<Uuid>,
    /// user uuid
    Uuid,
);

impl ProcessRequest<IsAuthorized> for AuthProcess {
    type Response = Option<Session>;

    fn handle(
        state: &mut Self::State,
        IsAuthorized(session_uuid, user_uuid): IsAuthorized,
    ) -> Self::Response {
        if let None = session_uuid {
            return None;
        }
        if let Some(session) = state.sessions.get(&session_uuid.unwrap()) {
            return match Instant::now().cmp(&session.expires_at) {
                Greater => Some(session.clone()),
                _ => None,
            };
        }
        // session doesn't exist
        return None;
    }
}
