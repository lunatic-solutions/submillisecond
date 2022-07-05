mod auth_manager;
mod session;
mod structures;

use http::HeaderMap;
use lunatic::process::{ProcessRef, Request};
use structures::*;
use submillisecond::{guard::Guard, handler::HandlerFn, json::Json, router};

pub use self::auth_manager::{AuthProcess, AuthResponse, AuthSup, SignupUserResponse};

/// signup
pub fn signup(Json(body): Json<Signup>) -> (HeaderMap, Json<SessionDataDto>) {
    let auth = ProcessRef::<AuthProcess>::lookup("auth_manager").unwrap();
    let mut headers = HeaderMap::new();
    match auth.request(body) {
        SignupUserResponse::Success(session) => {
            headers.insert(
                "Set-Cookie",
                format!("session={}", session.uuid).parse().unwrap(),
            );
            (
                headers,
                Json(SessionDataDto {
                    success: true,
                    message: "You are logged in".to_string(),
                }),
            )
        }
        SignupUserResponse::Failed(reason) => (
            headers,
            Json(SessionDataDto {
                success: true,
                message: "Invalid signup data".to_string(),
            }),
        ),
    }
}

/// login
pub fn login(Json(body): Json<LoginDto>) -> (HeaderMap, Json<SessionDataDto>) {
    let auth = ProcessRef::<AuthProcess>::lookup("auth_manager").unwrap();
    let mut headers = HeaderMap::new();
    match auth.request(body) {
        AuthResponse::LoggedIn(session) => {
            headers.insert(
                "Set-Cookie",
                format!("session={}", session.uuid).parse().unwrap(),
            );
            (
                headers,
                Json(SessionDataDto {
                    success: true,
                    message: "You are logged in".to_string(),
                }),
            )
        }
        _ => (
            headers,
            Json(SessionDataDto {
                success: true,
                message: "Invalid email and/or password".to_string(),
            }),
        ),
    }
}

/// logout
pub fn logout(Json(body): Json<LogoutDto>) {}

/// reset_password
pub fn reset_password(
    Json(body): Json<UpdatePasswordDto>,
) -> (http::StatusCode, Json<PasswordChangedDto>) {
    (
        http::StatusCode::NOT_IMPLEMENTED,
        Json(PasswordChangedDto { success: false }),
    )
}

/// update_password
pub fn update_password(
    Json(body): Json<ResetPasswordDto>,
) -> (http::StatusCode, Json<PasswordChangedDto>) {
    (
        http::StatusCode::NOT_IMPLEMENTED,
        Json(PasswordChangedDto { success: false }),
    )
}

// pub struct AuthGuard {}

// impl Guard for AuthGuard {
//     fn check(&self, req: &submillisecond::Request) -> bool {
//         // if req.uri().
//     }
// }

pub const AUTH_ROUTER: HandlerFn = router! {
    POST "/signup" => signup
    POST "/login" => login
    POST "/logout" => logout
    POST "/password/reset" => reset_password
    PUT "/password/" => update_password
};
