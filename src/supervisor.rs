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

/// The `RequestSupervisorProcess` is supervising one instance of a `RequestProcess`.
pub struct RequestSupervisorProcess {
    stream: TcpStream,
    this: ProcessRef<Self>,
    pointer_raw: usize,
    handler_process: Process<()>,
}

impl AbstractProcess for RequestSupervisorProcess {
    type Arg = (TcpStream, usize);
    type State = Self;

    fn init(this: ProcessRef<Self>, (stream, pointer_raw): Self::Arg) -> Self::State {
        // RequestSupervisorProcess shouldn't die when a request handler dies.
        unsafe { host::api::process::die_when_link_dies(1) };
        // spawn a new process that reads from the stream and sends back a message
        let handler_process = Process::spawn_link(
            (stream.clone(), pointer_raw, this.clone()),
            child_handler_process,
        );

        RequestSupervisorProcess {
            stream,
            this,
            pointer_raw,
            handler_process,
        }
    }

    fn terminate(_state: Self::State) {}

    fn handle_link_trapped(state: &mut Self::State, _tag: lunatic::Tag) {
        println!("Handling trapped link");
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
                    panic!("Failed to parse request {:?}", err);
                }
            };
        }
    };

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
