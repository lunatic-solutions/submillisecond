//! Websockets.

use std::ops;
use std::time::Duration;

use base64ct::{Base64, Encoding};
use http::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use http::StatusCode;
use lunatic::function::FuncRef;
use lunatic::net::TcpStream;
use lunatic::{Mailbox, Process};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
pub use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::Role;
pub use tungstenite::Message;

use crate::extract::FromRequest;
use crate::response::{IntoResponse, Response};
use crate::supervisor::{Connection, SupervisorResponse};
use crate::RequestContext;

/// A websocket connection.
#[derive(Debug)]
pub struct WebSocketConnection(tungstenite::protocol::WebSocket<TcpStream>);

impl From<tungstenite::protocol::WebSocket<TcpStream>> for WebSocketConnection {
    fn from(conn: tungstenite::protocol::WebSocket<TcpStream>) -> Self {
        WebSocketConnection(conn)
    }
}

impl WebSocketConnection {
    /// Splits this `WebSocketConnection` object into separate `Sink` and `Stream` objects.
    ///
    /// This can be useful when you want to split ownership between processes.
    pub fn split(self) -> (SplitSink, SplitStream) {
        (
            SplitSink {
                ws_conn: self.clone(),
            },
            SplitStream { ws_conn: self },
        )
    }
}

/// A `Sink` part of the split pair.
pub struct SplitSink {
    ws_conn: WebSocketConnection,
}

impl SplitSink {
    /// Read a message from stream, if possible.
    pub fn can_read(&self) -> bool {
        self.ws_conn.can_read()
    }

    /// Read a message from stream, if possible.
    pub fn read_message(&mut self) -> tungstenite::Result<Message> {
        self.ws_conn.read_message()
    }

    /// Close the connection.
    pub fn close(&mut self, code: Option<CloseFrame>) -> tungstenite::Result<()> {
        self.ws_conn.close(code)
    }
}

/// A Stream part of the split pair.
pub struct SplitStream {
    ws_conn: WebSocketConnection,
}

impl SplitStream {
    /// Check if it is possible to write messages.
    ///
    /// Writing gets impossible immediately after sending or receiving `Message::Close`.
    pub fn can_write(&self) -> bool {
        self.ws_conn.can_write()
    }

    /// Send a message to stream, if possible.
    pub fn write_message(&mut self, message: Message) -> tungstenite::Result<()> {
        self.ws_conn.write_message(message)
    }

    /// Flush the pending send queue.
    pub fn write_pending(&mut self) -> tungstenite::Result<()> {
        self.ws_conn.write_pending()
    }

    /// Close the connection.
    pub fn close(&mut self, code: Option<CloseFrame>) -> tungstenite::Result<()> {
        self.ws_conn.close(code)
    }
}

impl Clone for WebSocketConnection {
    fn clone(&self) -> Self {
        tungstenite::WebSocket::from_raw_socket(
            self.get_ref().clone(),
            Role::Server,
            Some(self.get_config().clone()),
        )
        .into()
    }
}

impl ops::Deref for WebSocketConnection {
    type Target = tungstenite::protocol::WebSocket<TcpStream>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for WebSocketConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Serialize for WebSocketConnection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut conn = serializer.serialize_struct("WebSocketConnection", 2)?;
        conn.serialize_field("stream", self.0.get_ref())?;
        conn.serialize_field("config", &WebSocketConfig::from(*self.0.get_config()))?;
        conn.end()
    }
}

impl<'de> Deserialize<'de> for WebSocketConnection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WebSocketConnectionVisitor {
            stream: TcpStream,
            config: WebSocketConfig,
        }

        let conn = WebSocketConnectionVisitor::deserialize(deserializer)?;

        Ok(tungstenite::protocol::WebSocket::from_raw_socket(
            conn.stream,
            Role::Server,
            Some(conn.config.into()),
        )
        .into())
    }
}

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
                    callback(conn.into());
                }
            },
        );

        WebSocketUpgrade {
            process,
            websocket_key: self.websocket_key,
        }
    }
}

impl FromRequest for WebSocket {
    type Rejection = WebSocketRejection;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        // Connection must be Upgrade
        let upgrade_header = req
            .headers()
            .get(CONNECTION)
            .ok_or(WebSocketRejection::MissingUpgradeHeader)?;
        if upgrade_header.as_bytes() != b"Upgrade" {
            return Err(WebSocketRejection::MissingUpgradeHeader);
        }

        // Upgrade must be websocket
        let upgrade_header = req
            .headers()
            .get(UPGRADE)
            .ok_or(WebSocketRejection::MissingUpgradeHeader)?;
        if upgrade_header.as_bytes() != b"websocket" {
            return Err(WebSocketRejection::MissingUpgradeHeader);
        }

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
            websocket_key,
        })
    }
}

/// An upgraded websocket response.
pub struct WebSocketUpgrade {
    process: Process<SupervisorResponse>,
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
            .extension(Connection::Upgrade(self.process))
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

impl From<tungstenite::protocol::WebSocketConfig> for WebSocketConfig {
    fn from(config: tungstenite::protocol::WebSocketConfig) -> Self {
        WebSocketConfig {
            max_send_queue: config.max_send_queue,
            max_message_size: config.max_message_size,
            max_frame_size: config.max_frame_size,
            accept_unmasked_frames: config.accept_unmasked_frames,
        }
    }
}

/// WebSocket upgrade rejection.
pub enum WebSocketRejection {
    /// Missing upgrade header.
    MissingUpgradeHeader,
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
            WebSocketRejection::MissingUpgradeHeader => {
                (StatusCode::BAD_REQUEST, "missing upgrade header")
            }
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
