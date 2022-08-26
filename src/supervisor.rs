use std::io::Write;
use std::time::Duration;

use headers::HeaderValue;
use http::{header, Request, StatusCode, Version};
use lunatic::function::reference::Fn;
use lunatic::function::FuncRef;
use lunatic::net::TcpStream;
use lunatic::{Mailbox, Process};
use serde::{Deserialize, Serialize};

use crate::core::Body;
use crate::response::{IntoResponse, Response};
use crate::{core, Handler, RequestContext};

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub(crate) struct WorkerRequest<T>
where
    T: Fn<T>,
{
    supervisor: Process<WorkerResponse>,
    stream: TcpStream,
    handler: FuncRef<T>,
    #[serde(with = "serde_bytes")]
    request_buffer: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum WorkerResponse {
    Failure(String),
    /// Response contains the HTTP response and sometimes data from a pipelined
    /// request.
    Response(
        #[serde(with = "serde_bytes")] Vec<u8>,
        #[serde(with = "serde_bytes")] Vec<u8>,
    ),
}

pub(crate) fn request_supervisor<T, Arg, Ret>(
    (mut stream, handler_ref): (TcpStream, FuncRef<T>),
    mailbox: Mailbox<WorkerResponse>,
) where
    T: Handler<Arg, Ret> + Copy,
    T: Fn<T> + Copy,
{
    let supervisor = mailbox.this();
    let mut request_buffer: Vec<u8> = Vec::new();

    // Failure in linked worker processes should not kill the supervisor
    let mailbox = mailbox.catch_link_failure();

    // keep-alive loop
    'keepalive: loop {
        // Spawn worker process
        let worker = Process::spawn_link(
            WorkerRequest {
                supervisor: supervisor.clone(),
                stream: stream.clone(),
                handler: handler_ref,
                request_buffer,
            },
            request_woker::<T, Arg, Ret>,
        );

        // Each request has a default 5 minute timeout.
        match mailbox.receive_timeout(Duration::from_secs(5 * 60)) {
            lunatic::MailboxResult::Message(msg) => match msg {
                WorkerResponse::Response(ref data, next) => {
                    let result = stream.write_all(data);
                    if let Err(err) = result {
                        log_error(&format!("Failed to send response: {:?}", err));
                        break 'keepalive;
                    }
                    request_buffer = next;
                }
                WorkerResponse::Failure(ref err) => {
                    log_error(err);
                    let response: Response =
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
                            .into_response();
                    let response = response_to_vec(response);
                    let result = stream.write_all(&response);
                    if let Err(err) = result {
                        log_error(&format!("Failed to send response: {:?}", err));
                    }
                    break 'keepalive;
                }
            },
            lunatic::MailboxResult::TimedOut => {
                // Kill worker
                worker.kill();
                log_error(&String::from("Request timed out"));
                let response: Response =
                    (StatusCode::REQUEST_TIMEOUT, "Request timed out").into_response();
                let response = response_to_vec(response);
                let result = stream.write_all(&response);
                if let Err(err) = result {
                    log_error(&format!("Failed to send response: {:?}", err));
                }
                break 'keepalive;
            }
            lunatic::MailboxResult::LinkDied(_) => {
                log_error(&String::from("Worker process panicked"));
                let response: Response =
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
                let response = response_to_vec(response);
                let result = stream.write_all(&response);
                if let Err(err) = result {
                    log_error(&format!("Failed to send response: {:?}", err));
                }
                break 'keepalive;
            }
            _ => unreachable!(),
        };
    }
}

/// Request workers are processes that do all the request parsing and handling.
/// At the end they return a buffer to the supervisor to send the response back
/// to the client.
fn request_woker<T, Arg, Ret>(worker_request: WorkerRequest<T>, _: Mailbox<()>)
where
    T: Handler<Arg, Ret> + Copy,
    T: Fn<T> + Copy,
{
    let handler = *worker_request.handler;
    let mut requests_buffer = worker_request.request_buffer;
    // SAFETY:
    //
    // The following call will extend the lifetime of `&mut requests_buffer`
    // to `'static`.
    //
    // This is necessary because we want all references to it to also have the
    // `'static` lifetime. It significantly reduces type complexity and allows the
    // `RequestContext` to avoid a lifetimes too. Because of Rust's limitations we
    // can't implement the `Handler` trait for the type `fn(RequestContext<'a>)`,
    // only for `fn(RequestContext)`. This was the simplest workaround for it.
    //
    // It is actually safe to dot this. This function is an entry point to a process
    // and `requests_buffer` will only be dropped right before the process finishes.
    let requests_buffer = unsafe { std::mem::transmute(&mut requests_buffer) };

    let pipelined_request = core::parse_requests(requests_buffer, worker_request.stream);

    let (request, next) = pipelined_request.pipeline();
    // Check if first request is valid
    let request = match request {
        Ok(request) => request,
        Err(error) => {
            worker_request
                .supervisor
                .send(WorkerResponse::Failure(format!(
                    "Reqeust parsing failed: {:?}",
                    error
                )));
            return; // Abort request handling
        }
    };

    log_request(&request);

    let response = Handler::handle(&handler, RequestContext::from(request)).into_response();
    let response = response_to_vec(response);
    worker_request
        .supervisor
        .send(WorkerResponse::Response(response, next));
}

fn response_to_vec(mut response: Response) -> Vec<u8> {
    let mut response_buffer = Vec::new();

    let content_length = response.body().len();
    *response.version_mut() = Version::HTTP_11;
    response
        .headers_mut()
        .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

    // writing status line
    response_buffer.extend(
        format!(
            "{:?} {} {}\r\n",
            response.version(),
            response.status().as_u16(),
            response.status().canonical_reason().unwrap()
        )
        .as_bytes(),
    );
    // writing headers
    for (key, value) in response.headers().iter() {
        if let Ok(value) = String::from_utf8(value.as_ref().to_vec()) {
            response_buffer.extend(format!("{}: {}\r\n", key, value).as_bytes());
        }
    }
    // separator between header and data
    response_buffer.extend("\r\n".as_bytes());
    response_buffer.extend(response.body());

    response_buffer
}

#[cfg(feature = "logging")]
fn log_request(request: &Request<Body>) {
    let method_string = match *request.method() {
        http::Method::GET => ansi_term::Color::Green.normal(),
        http::Method::POST => ansi_term::Color::Blue.normal(),
        http::Method::PUT => ansi_term::Color::Yellow.normal(),
        http::Method::DELETE => ansi_term::Color::Red.normal(),
        http::Method::HEAD => ansi_term::Color::Purple.normal(),
        http::Method::OPTIONS => ansi_term::Color::Blue.dimmed(),
        http::Method::PATCH => ansi_term::Color::Cyan.normal(),
        _ => ansi_term::Color::White.normal(),
    }
    .bold()
    .paint(request.method().as_str());

    let ip = ansi_term::Style::new().dimmed().paint(
        request
            .headers()
            .get(http::header::HeaderName::from_static("x-forwarded-for"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-"),
    );

    lunatic_log::info!("{} {}    {}", method_string, request.uri(), ip);
}

#[cfg(not(feature = "logging"))]
fn log_request(_request: &Request<Body>) {}

#[cfg(feature = "logging")]
fn log_error(err: &String) {
    lunatic_log::error!("{}", err);
}

#[cfg(not(feature = "logging"))]
fn log_error(_err: &String) {}
