use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Signup {
    pub email: String,
    pub password: String,
    pub nickname: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LoginDto {
    pub nickname: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LogoutDto {
    pub access_token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpdatePasswordDto {
    pub old_pw: String,
    pub new_pw: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ResetPasswordDto {
    pub new_pw: String,
    pub reset_code: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SessionDataDto {
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PasswordChangedDto {
    pub success: bool,
}
