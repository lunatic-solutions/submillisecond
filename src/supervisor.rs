use crate::core;
use crate::response::IntoResponse;
use crate::router::Route;
use crate::{handler::HandlerFn, Response};
use http::{header, HeaderValue};
use lunatic::process::Message;
use lunatic::{
    host,
    net::TcpStream,
    process::{AbstractProcess, ProcessMessage, ProcessRef},
    Mailbox, Process,
};
use std::borrow::Cow;
use std::io::Write;
use std::mem;

/// The `RequestSupervisorProcess` is supervising one instance of a `RequestProcess`.
pub struct RequestSupervisorProcess {
    stream: TcpStream,
}

impl AbstractProcess for RequestSupervisorProcess {
    type Arg = (TcpStream, usize);
    type State = Self;

    fn init(this: ProcessRef<Self>, (stream, pointer_raw): Self::Arg) -> Self::State {
        // RequestSupervisorProcess shouldn't die when a request handler dies.
        unsafe { host::api::process::die_when_link_dies(1) };
        // spawn a new process that reads from the stream and sends back a message
        let handler_process =
            Process::spawn_link((stream.clone(), pointer_raw, this), child_handler_process);

        // link handler process such that handle_link_trapped is called
        handler_process.link();

        RequestSupervisorProcess { stream }
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

    let mut request = match core::parse_request(stream.clone()) {
        Ok(request) => request,
        Err(err) => {
            return match core::encode_response(err.into_response()) {
                Ok(output) => parent.send(output),
                Err(err) => {
                    eprintln!("[http reader] Failed to send response {:?}", err);
                    panic!("Failed to parse request {:?}", err);
                }
            };
        }
    };

    let path = request.uri().path().to_string();
    let extensions = request.extensions_mut();
    extensions.insert(Route(Cow::Owned(path.clone())));
    let http_version = request.version();

    let params = crate::params::Params::new();
    let reader = crate::core::UriReader::new(path);
    let mut response = handler(request, params, reader).unwrap_or_else(|err| err.into_response());

    let content_length = response.body().len();
    *response.version_mut() = http_version;
    response
        .headers_mut()
        .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

    match core::encode_response(response) {
        Ok(output) => parent.send(output),
        Err(err) => eprintln!("[http reader] Failed to send response {:?}", err),
    }
}

impl ProcessMessage<Vec<u8>> for RequestSupervisorProcess {
    fn handle(state: &mut RequestSupervisorProcess, response: Vec<u8>) {
        if let Err(err) = state.stream.write_all(&response) {
            eprintln!("[http reader] Failed to send response {:?}", err);
        }
        // Not a cool way imo
        panic!("FINISHED RESPONSE");
    }
}
