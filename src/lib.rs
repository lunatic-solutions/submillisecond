use std::{
    io::{self},
    mem,
};

use handler::HandlerFn;
use http::{header, HeaderValue};
use lunatic::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    Mailbox, Process,
};
use response::IntoResponse;
pub use submillisecond_macros::*;

pub use crate::error::{BoxError, Error};
pub use crate::response::Response;
use crate::router::Route;

#[macro_use]
pub(crate) mod macros;

pub mod core;
pub mod defaults;
mod error;
pub mod extract;
pub mod handler;
pub mod json;
pub mod response;
pub mod router;
pub mod template;

/// Type alias for [`http::Request`] whose body defaults to [`String`].
pub type Request<T = Vec<u8>> = http::Request<T>;

#[derive(Clone, Copy, Debug)]
pub struct Application {
    router: HandlerFn,
}

impl Application {
    pub fn new(router: HandlerFn) -> Self {
        Application { router }
    }

    pub fn serve<A: ToSocketAddrs>(self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;

        while let Ok((stream, _)) = listener.accept() {
            Process::spawn_link(
                (stream, self.router as *const () as usize),
                |(stream, handler_raw): (TcpStream, usize), _: Mailbox<()>| {
                    let handler = unsafe {
                        let pointer = handler_raw as *const ();
                        mem::transmute::<*const (), HandlerFn>(pointer)
                    };

                    let mut request = match core::parse_request(stream.clone()) {
                        Ok(request) => request,
                        Err(err) => {
                            if let Err(err) = core::write_response(stream, err.into_response()) {
                                eprintln!("[http reader] Failed to send response {:?}", err);
                            }
                            return;
                        }
                    };

                    let path = request.uri().path().to_string();
                    let extensions = request.extensions_mut();
                    extensions.insert(Route(path));
                    let http_version = request.version();

                    let mut response = handler(request).unwrap_or_else(|err| err.into_response());

                    let content_length = response.body().len();
                    *response.version_mut() = http_version;
                    response
                        .headers_mut()
                        .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

                    if let Err(err) = core::write_response(stream, response) {
                        eprintln!("[http reader] Failed to send response {:?}", err);
                    }
                },
            );
        }

        Ok(())
    }
}
