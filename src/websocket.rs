//! Websockets.

use std::time::Duration;

use base64ct::{Base64, Encoding};
use http::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use http::StatusCode;
use lunatic::function::FuncRef;
use lunatic::net::TcpStream;
use lunatic::{Mailbox, Process};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
pub use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::Role;
pub use tungstenite::Message;

use crate::extract::FromRequest;
use crate::response::{IntoResponse, Response};
use crate::supervisor::{SupervisorResponse, WorkerResponse};
use crate::RequestContext;

/// A websocket connection.
pub type WebSocketConnection = tungstenite::protocol::WebSocket<TcpStream>;

/// WebSocket extractor which upgrades a HTTP connection.
///
/// # Example
///
/// ```
/// fn websocket(ws: WebSocket) -> WebSocketUpgrade {
///     ws.on_upgrade(|conn| {
///         conn.write_message(Message::text("Hello from submillisecond!"));
///     })
/// }
///
/// router! {
///     GET "/ws" => websocket
/// }
/// ```
pub struct WebSocket {
    stream: TcpStream,
    supervisor: Process<WorkerResponse>,
    websocket_key: Vec<u8>,
}

impl WebSocket {
    /// Spawns a process with the established websocket connection provided to
    /// the callback.
    pub fn on_upgrade(self, callback: fn(WebSocketConnection)) -> WebSocketUpgrade {
        self.on_upgrade_with_config(callback, None)
    }

    /// Spawns a process with the established websocket connection provided to
    /// the callback with websocket options.
    pub fn on_upgrade_with_config(
        self,
        callback: fn(WebSocketConnection),
        config: Option<WebSocketConfig>,
    ) -> WebSocketUpgrade {
        let stream = self.stream.clone();
        let callback = FuncRef::new(callback);
        let process = Process::spawn_link(
            (stream, callback, config),
            |(stream, callback, config), mailbox: Mailbox<SupervisorResponse>| {
                if let lunatic::MailboxResult::Message(SupervisorResponse::ResponseSent) =
                    mailbox.receive_timeout(Duration::from_secs(5))
                {
                    let conn = tungstenite::protocol::WebSocket::from_raw_socket(
                        stream,
                        Role::Server,
                        config.map(|config| config.into()),
                    );
                    callback(conn);
                }
            },
        );

        self.supervisor
            .send(WorkerResponse::UpdateWorkerProcess(process));

        WebSocketUpgrade {
            websocket_key: self.websocket_key,
        }
    }
}

impl FromRequest for WebSocket {
    type Rejection = WebSocketRejection;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        // Must be HTTP 1.1 or greater
        if req.version() < http::Version::HTTP_11 {
            return Err(WebSocketRejection::UnsupportedHttpVersion);
        }

        // Must be GET for websocket
        if req.method() != http::Method::GET {
            return Err(WebSocketRejection::UnsupportedHttpMethod);
        }

        // Websocket version must be 13
        let websocket_version = req
            .headers()
            .get(SEC_WEBSOCKET_VERSION)
            .ok_or(WebSocketRejection::MissingWebSocketVersion)?;
        if websocket_version.as_bytes() != b"13" {
            return Err(WebSocketRejection::UnsupportedWebSocketVersion);
        }

        let websocket_key = req
            .headers()
            .get(SEC_WEBSOCKET_KEY)
            .ok_or(WebSocketRejection::MissingWebSocketKey)?
            .as_bytes()
            .to_owned();

        Ok(WebSocket {
            stream: req.stream.clone(),
            supervisor: req.supervisor.clone(),
            websocket_key,
        })
    }
}

/// An upgraded websocket response.
pub struct WebSocketUpgrade {
    websocket_key: Vec<u8>,
}

impl IntoResponse for WebSocketUpgrade {
    fn into_response(mut self) -> Response {
        self.websocket_key
            .extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

        let mut hasher = Sha1::new();
        hasher.update(&self.websocket_key);
        let key = hasher.finalize();
        let base64_key = Base64::encode_string(&key);

        Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "Upgrade")
            .header(SEC_WEBSOCKET_ACCEPT, base64_key)
            .body(Vec::new())
            .unwrap()
    }
}

/// The configuration for WebSocket connection.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// The size of the send queue. You can use it to turn on/off the
    /// backpressure features. `None` means here that the size of the queue
    /// is unlimited. The default value is the unlimited queue.
    pub max_send_queue: Option<usize>,
    /// The maximum size of a message. `None` means no size limit. The default
    /// value is 64 MiB which should be reasonably big for all normal
    /// use-cases but small enough to prevent memory eating by a malicious
    /// user.
    pub max_message_size: Option<usize>,
    /// The maximum size of a single message frame. `None` means no size limit.
    /// The limit is for frame payload NOT including the frame header. The
    /// default value is 16 MiB which should be reasonably big for all
    /// normal use-cases but small enough to prevent memory eating
    /// by a malicious user.
    pub max_frame_size: Option<usize>,
    /// When set to `true`, the server will accept and handle unmasked frames
    /// from the client. According to the RFC 6455, the server must close the
    /// connection to the client in such cases, however it seems like there are
    /// some popular libraries that are sending unmasked frames, ignoring the
    /// RFC. By default this option is set to `false`, i.e. according to RFC
    /// 6455.
    pub accept_unmasked_frames: bool,
}

impl From<WebSocketConfig> for tungstenite::protocol::WebSocketConfig {
    fn from(config: WebSocketConfig) -> Self {
        tungstenite::protocol::WebSocketConfig {
            max_send_queue: config.max_send_queue,
            max_message_size: config.max_message_size,
            max_frame_size: config.max_frame_size,
            accept_unmasked_frames: config.accept_unmasked_frames,
        }
    }
}

/// WebSocket upgrade rejection.
pub enum WebSocketRejection {
    /// Missing websocket key.
    MissingWebSocketKey,
    /// Missing websocket version.
    MissingWebSocketVersion,
    /// Unsupported HTTP version.
    UnsupportedHttpVersion,
    /// Unsupported HTTP method.
    UnsupportedHttpMethod,
    /// Unsupported websocket version.
    UnsupportedWebSocketVersion,
}

impl IntoResponse for WebSocketRejection {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            WebSocketRejection::MissingWebSocketKey => {
                (StatusCode::BAD_REQUEST, "missing websocket key")
            }
            WebSocketRejection::MissingWebSocketVersion => {
                (StatusCode::BAD_REQUEST, "missing websocket version")
            }
            WebSocketRejection::UnsupportedHttpVersion => (
                StatusCode::HTTP_VERSION_NOT_SUPPORTED,
                "unsupported http version for websocket",
            ),
            WebSocketRejection::UnsupportedHttpMethod => (
                StatusCode::METHOD_NOT_ALLOWED,
                "http method not allowed for websocket",
            ),
            WebSocketRejection::UnsupportedWebSocketVersion => (
                StatusCode::NOT_IMPLEMENTED,
                "websocket version not supported",
            ),
        };

        (status, body).into_response()
    }
}