use std::io::Result as IoResult;

use http::{header, HeaderValue};
use lunatic::{
    net::{TcpListener, TcpStream},
    Mailbox, Process,
};
use response::IntoResponse;
pub use submillisecond_macros::*;

pub use crate::error::{BoxError, Error};
pub use crate::response::Response;
use crate::router::{HandlerFn, Route, Router};

#[macro_use]
pub(crate) mod macros;

pub mod core;
pub mod defaults;
mod error;
pub mod extract;
pub mod json;
pub mod response;
pub mod router;
pub mod template;

/// Type alias for [`http::Request`] whose body defaults to [`String`].
pub type Request<T = Vec<u8>> = http::Request<T>;

pub struct Application {
    listener: TcpListener,
    router: Router,
}

pub struct ApplicationBuilder {
    router: Router,
}

impl ApplicationBuilder {
    pub fn route(mut self, handler: HandlerFn) -> ApplicationBuilder {
        self.router.route(handler);
        self
    }

    pub fn listen(self, port: u32) -> IoResult<Application> {
        match TcpListener::bind(format!("0.0.0.0:{}", port)) {
            Ok(listener) => Ok(Application {
                listener,
                router: self.router,
            }),
            Err(e) => Err(e),
        }
    }
}

impl Application {
    pub fn build() -> ApplicationBuilder {
        ApplicationBuilder {
            router: Router::new(),
        }
    }

    pub fn start_server(self) {
        while let Ok((stream, _)) = self.listener.accept() {
            Process::spawn_link(
                (stream, self.router.as_raw()),
                |(stream, raw): (TcpStream, Vec<usize>), _: Mailbox<()>| {
                    let router = Router::from_raw(raw);
                    let mut request = match core::parse_request(stream.clone()) {
                        Ok(request) => request,
                        Err(err) => {
                            if let Err(err) = core::write_response(stream, err.into_response()) {
                                eprintln!("[http reader] Failed to send response {:?}", err);
                            }
                            return;
                        }
                    };
                    let path_and_query = request.uri().path_and_query().cloned().unwrap();
                    let extensions = request.extensions_mut();
                    extensions.insert(Route::new(path_and_query));
                    let http_version = request.version();
                    let mut response = router.handle_request(request);
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
    }
}
