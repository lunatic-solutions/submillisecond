use std::io::Write;

use headers::HeaderValue;
use http::{header, Request};
use lunatic::function::reference::Fn;
use lunatic::function::FuncRef;
use lunatic::net::TcpStream;
use lunatic::{Mailbox, Process};
use serde::{Deserialize, Serialize};

use crate::response::IntoResponse;
use crate::{core, Handler, RequestContext};

#[derive(Serialize, Deserialize)]
pub(crate) enum WorkerResponse {
    Failure(String),
    #[serde(with = "serde_bytes")]
    Response(Vec<u8>),
    FinishPipeline,
}

pub(crate) fn request_supervisor<T, Arg, Ret>(
    (mut stream, handler_ref): (TcpStream, FuncRef<T>),
    mailbox: Mailbox<WorkerResponse>,
) where
    T: Fn<T> + Copy,
    T: Handler<Arg, Ret>,
{
    let supervisor = mailbox.this();

    // keep-alive loop
    'keepalive: loop {
        // Spawn worker process
        Process::spawn(
            (supervisor.clone(), stream.clone(), handler_ref),
            request_woker::<T, Arg, Ret>,
        );

        'pipeline: loop {
            match mailbox.receive() {
                WorkerResponse::Response(ref data) => {
                    let result = stream.write_all(data);
                    if let Err(err) = result {
                        log_error(&format!("Failed to send response: {:?}", err));
                        break 'keepalive;
                    }
                }
                WorkerResponse::Failure(ref err) => {
                    log_error(err);
                    break 'keepalive;
                }
                WorkerResponse::FinishPipeline => break 'pipeline,
            };
        }
    }
}

fn request_woker<T, Arg, Ret>(
    (supervisor, stream, handler_ref): (Process<WorkerResponse>, TcpStream, FuncRef<T>),
    _: Mailbox<()>,
) where
    T: Fn<T> + Copy,
    T: Handler<Arg, Ret>,
{
    let handler = *handler_ref;

    let mut requests_buffer = Vec::new();
    for request in core::parse_requests(&mut requests_buffer, stream)
        .request_results()
        .into_iter()
    {
        let request = match request {
            Ok(request) => request,
            Err(err) => {
                supervisor.send(WorkerResponse::Failure(format!(
                    "Reqeust parsing failed: {:?}",
                    err
                )));
                return;
            }
        };

        log_request(&request);

        let http_version = request.version();
        let mut response = Handler::handle(&handler, RequestContext::from(request)).into_response();

        let content_length = response.body().len();
        *response.version_mut() = http_version;
        response
            .headers_mut()
            .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

        let mut buffer = Vec::new();
        // writing status line
        buffer.extend(
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
                buffer.extend(format!("{}: {}\r\n", key, value).as_bytes());
            }
        }
        // separator between header and data
        buffer.extend("\r\n".as_bytes());
        buffer.extend(response.body());
        supervisor.send(WorkerResponse::Response(buffer));
    }
    supervisor.send(WorkerResponse::FinishPipeline);
}

#[cfg(feature = "logging")]
fn log_request(request: &Request<Vec<u8>>) {
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
fn log_request(_request: &Request<Vec<u8>>) {}

#[cfg(feature = "logging")]
fn log_error(err: &String) {
    lunatic_log::error!("{}", err);
}

#[cfg(not(feature = "logging"))]
fn log_error(_err: &String) {}
