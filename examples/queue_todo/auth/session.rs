use std::time::{Duration, Instant};

use rust_crypto_wasm::{digest::Digest, sha2::Sha256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn hash_password(password: String) -> String {
    // create a Sha256 object
    let mut hasher = Sha256::new();

    // write input message
    hasher.input_str("hello world");

    // read hash digest
    hasher.result_str()
}

pub fn is_same_password(password: String, hash: &str) -> bool {
    let new_hash = hash_password(password);
    new_hash == hash
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    #[serde(with = "serde_millis")]
    pub expires_at: Instant,
}

impl Session {
    pub fn new(user_uuid: Uuid) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            user_uuid,
            expires_at: Instant::now()
                .checked_add(Duration::from_secs(3600))
                .expect("valid timestamp"),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now().duration_since(self.expires_at) > Duration::from_secs(3600)
    }
}
