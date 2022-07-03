use crate::core;
use crate::response::IntoResponse;
use crate::router::Route;
use crate::{handler::HandlerFn, Response};
use http::{header, HeaderValue};
use lunatic::{
    host,
    net::TcpStream,
    process::{AbstractProcess, Message, ProcessMessage, ProcessRef},
    Mailbox, Process,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::Write;
use std::mem;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone)]
pub struct RequestProcessConfig {
    /// the duration after which a request should be closed due to timeout
    /// if the connection has not received a request yet
    /// defaults to 60 seconds
    max_request_duration: Duration,
}

impl Default for RequestProcessConfig {
    fn default() -> Self {
        Self {
            max_request_duration: Duration::new(60, 0),
        }
    }
}

/// The `RequestSupervisorProcess` is supervising one instance of a `RequestProcess`.
pub struct RequestSupervisorProcess {
    stream: TcpStream,
    this: ProcessRef<Self>,
    pointer_raw: usize,
    handler_process: Process<()>,
    last_request: Option<Instant>,
    config: RequestProcessConfig,
}

impl AbstractProcess for RequestSupervisorProcess {
    type Arg = (TcpStream, usize, RequestProcessConfig);
    type State = Self;

    fn init(this: ProcessRef<Self>, (stream, pointer_raw, config): Self::Arg) -> Self::State {
        // RequestSupervisorProcess shouldn't die when a request handler dies.
        unsafe { host::api::process::die_when_link_dies(1) };
        // spawn a new process that reads from the stream and sends back a message
        let handler_process = Process::spawn_link(
            (stream.clone(), pointer_raw, this.clone()),
            child_handler_process,
        );

        this.send_after(CloseConnection::Timeout, config.max_request_duration);

        RequestSupervisorProcess {
            stream,
            this,
            pointer_raw,
            handler_process,
            last_request: None,
            config,
        }
    }

    fn terminate(_state: Self::State) {}

    fn handle_link_trapped(state: &mut Self::State, _tag: lunatic::Tag) {
        match Response::builder()
            .status(500)
            .body("Internal Server Error".as_bytes().to_vec())
        {
            Ok(response) => match core::encode_response(response) {
                Ok(output) => state.stream.write_all(&output).unwrap(),
                Err(err) => {
                    eprintln!("[http reader] Failed to send response {:?}", err);
                    panic!("Failed to parse request {:?}", err);
                }
            },
            Err(e) => eprintln!("Failed to send error with code 500 {:?}", e),
        };
    }
}

fn child_handler_process(
    (stream, handler_raw, parent): (TcpStream, usize, ProcessRef<RequestSupervisorProcess>),
    _: Mailbox<()>,
) {
    let handler = unsafe {
        let pointer = handler_raw as *const ();
        mem::transmute::<*const (), HandlerFn>(pointer)
    };

    let mut keep_alive = false;
    let mut request = match core::parse_request(stream.clone()) {
        Ok(request) => request,
        Err(err) => {
            return match core::encode_response(err.into_response()) {
                Ok(output) => parent.send(ChildResponse(output, keep_alive)),
                Err(err) => {
                    eprintln!("[http reader] Failed to send response {:?}", err);
                    return parent.send(CloseConnection::End);
                }
            };
        }
    };

    parent.send(ReceivedRequest);

    if let Some(conn) = request.headers().get("Connection") {
        keep_alive = conn == "keep-alive";
    }

    let path = request.uri().path().to_string();
    let extensions = request.extensions_mut();
    extensions.insert(Route(Cow::Owned(path.clone())));

    let path = request.uri().path().to_string();
    let extensions = request.extensions_mut();
    extensions.insert(Route(Cow::Owned(path.clone())));
    let http_version = request.version();

    let params = crate::params::Params::new();
    let reader = crate::uri_reader::UriReader::new(path);
    let mut response = handler(request, params, reader).unwrap_or_else(|err| err.into_response());

    let content_length = response.body().len();
    *response.version_mut() = http_version;
    response
        .headers_mut()
        .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

    match core::encode_response(response) {
        Ok(output) => parent.send(ChildResponse(output, keep_alive)),
        Err(err) => eprintln!("[http reader] Failed to send response {:?}", err),
    }
}

#[derive(Serialize, Deserialize)]
struct ChildResponse(
    /// encoded http response of child process
    #[serde(with = "serde_bytes")]
    Vec<u8>,
    /// boolean indicating whether connection should be reused
    bool,
);
impl ProcessMessage<ChildResponse> for RequestSupervisorProcess {
    fn handle(
        state: &mut RequestSupervisorProcess,
        ChildResponse(response, keep_alive): ChildResponse,
    ) {
        if let Err(err) = state.stream.write_all(&response) {
            eprintln!("[http reader] Failed to send response {:?}", err);
        }
        if keep_alive {
            // unlink from finished handler
            state.handler_process.unlink();
            // create new process
            state.handler_process = Process::spawn_link(
                (state.stream.clone(), state.pointer_raw, state.this.clone()),
                child_handler_process,
            );
        } else {
            state.this.shutdown()
        }
    }
}

#[derive(Serialize, Deserialize)]
enum CloseConnection {
    Timeout,
    End,
}
impl ProcessMessage<CloseConnection> for RequestSupervisorProcess {
    fn handle(state: &mut RequestSupervisorProcess, reason: CloseConnection) {
        if let CloseConnection::Timeout = reason {
            if let Some(last) = state.last_request.map(|l| Instant::now().duration_since(l)) {
                // last response was sent not that long ago, call send_after again
                if last < state.config.max_request_duration {
                    state.this.send_after(
                        CloseConnection::Timeout,
                        state.config.max_request_duration - last,
                    );
                    return;
                }
            }
            // kill handler
            state.handler_process.unlink();
            state.handler_process.kill();
            println!("Closing connection due to timeout");
            // unwrap builder result here because there's something really bad going on if this fails
            return match core::encode_response(
                Response::builder().status(408).body(vec![]).unwrap(),
            ) {
                Ok(response) => {
                    if let Err(e) = state.stream.write_all(&response) {
                        eprintln!("Failed to send 408 timeout to client {:?}", e);
                    }
                    return state.this.shutdown();
                }
                Err(enc) => eprintln!("Failed to encode 408 response {:?}", enc),
            };
        }
        state.handler_process.unlink();

        println!("Shutting down supervisor...");
        state.this.shutdown()
    }
}

#[derive(Serialize, Deserialize)]
struct ReceivedRequest;
impl ProcessMessage<ReceivedRequest> for RequestSupervisorProcess {
    fn handle(state: &mut RequestSupervisorProcess, _: ReceivedRequest) {
        state.last_request = Some(Instant::now());
    }
}
