//! Session data stored in encrypted user cookie.
//!
//! Sessions can be shared across handlers as long as the same type in the
//! generic is used. Using different session types across handlers is valid,
//! and the data will be stored in separate cookies named
//! `session-{type_id_hash}`.
//!
//! The [`init_session`] should be called before starting the web server to
//! initialize a key.
//!
//! The [`cookies::cookies_layer`](super::cookies::cookies_layer) layer must
//! also be used for session data to work.
//!
//! # Example
//!
//! ```
//! use std::io;
//!
//! use submillisecond::cookies::{cookies_layer, Key};
//! use submillisecond::session::{init_session, Session};
//! use submillisecond::{router, Application};
//!
//! fn counter(mut session: Session<i32>) -> String {
//!     if *session < 10 {
//!         *session += 1;
//!     }
//!     session.to_string()
//! }
//!
//! fn main() -> io::Result<()> {
//!     session::init_session(Key::generate());
//!
//!     Application::new(router! {
//!         with cookies_layer;
//!
//!         GET "/counter" => counter
//!     })
//!     .serve("0.0.0.0:3000")
//! }
//! ```

use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

use cookie::{Cookie, Key};
use lunatic::process::{AbstractProcess, ProcessRef, Request, RequestHandler, StartProcess};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::cookies::COOKIES;
use crate::extract::FromRequest;

/// Initialize the session key.
pub fn init_session(key: Key) {
    SessionProcess::start(KeyWrapper(key), Some("submillisecond_session"));
}

/// Session extractor, used to store data encrypted in a browser cookie.
///
/// If the session does not exist from the request, a default session will be
/// used with [`Default::default`].
pub struct Session<D>
where
    D: Default + Serialize + DeserializeOwned + 'static,
{
    changed: bool,
    data: D,
    key: Key,
}

impl<D> Deref for Session<D>
where
    D: Default + Serialize + DeserializeOwned,
{
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<D> DerefMut for Session<D>
where
    D: Default + Serialize + DeserializeOwned,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.data
    }
}

impl<D> Drop for Session<D>
where
    D: Default + Serialize + DeserializeOwned + 'static,
{
    fn drop(&mut self) {
        if self.changed {
            if let Ok(value) = serde_json::to_string(&self.data) {
                COOKIES.with_borrow_mut(|mut cookies| {
                    let mut private_jar = cookies.private_mut(&self.key);
                    let cookie_name = cookie_name::<D>();
                    private_jar.add(Cookie::new(cookie_name, value));
                });
            }
        }
    }
}

impl<D> FromRequest for Session<D>
where
    D: Default + Serialize + DeserializeOwned + 'static,
{
    type Rejection = SessionProcessNotRunning;

    fn from_request(_req: &mut crate::RequestContext) -> Result<Self, Self::Rejection> {
        let session_process = ProcessRef::<SessionProcess>::lookup("submillisecond_session")
            .ok_or(SessionProcessNotRunning)?;
        let KeyWrapper(key) = session_process.request(GetSessionNameKey);
        let cookie_name = cookie_name::<D>();
        let (changed, data) = COOKIES.with_borrow(|cookies| {
            let private_jar = cookies.private(&key);
            let session_cookie = private_jar.get(&cookie_name);
            let changed = session_cookie.is_none();
            let data = session_cookie
                .and_then(|session_cookie| serde_json::from_str(session_cookie.value()).ok())
                .unwrap_or_default();
            (changed, data)
        });
        Ok(Session { changed, data, key })
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Session key not configured. Did you forget to call `session::init_session`?"]
    /// Rejection type used when the session process has not been started via [`session::init_session`].
    pub struct SessionProcessNotRunning;
}

struct SessionProcess(KeyWrapper);
impl AbstractProcess for SessionProcess {
    type Arg = KeyWrapper;
    type State = Self;

    fn init(_: ProcessRef<Self>, key: KeyWrapper) -> Self::State {
        Self(key)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct GetSessionNameKey;
impl RequestHandler<GetSessionNameKey> for SessionProcess {
    type Response = KeyWrapper;

    fn handle(state: &mut Self::State, _: GetSessionNameKey) -> Self::Response {
        state.0.clone()
    }
}

#[derive(Clone)]
struct KeyWrapper(Key);

impl Serialize for KeyWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.master())
    }
}

impl<'de> Deserialize<'de> for KeyWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let key = <Vec<u8>>::deserialize(deserializer)?;
        Ok(KeyWrapper(Key::from(&key)))
    }
}

fn cookie_name<D: 'static>() -> String {
    let type_id = TypeId::of::<D>();
    let mut hasher = DefaultHasher::new();
    type_id.hash(&mut hasher);
    let hash = hasher.finish();
    format!("session-{hash:x}")
}
