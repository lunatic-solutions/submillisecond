use std::io::Result as IoResult;

pub use http::Method;
use lunatic::{
    net::{TcpListener, TcpStream},
    Mailbox, Process,
};
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
                    let mut request = core::parse_request(stream.clone());
                    let path_and_query = request.uri().path_and_query().cloned().unwrap();
                    let extensions = request.extensions_mut();
                    extensions.insert(Route::new(path_and_query));
                    let http_version = request.version();
                    let response = router.handle_request(request);
                    let res = Response::builder()
                        .version(http_version)
                        .header("content-length", response.body().len())
                        .header("content-type", "text/html")
                        .status(200)
                        .body(response.into_body())
                        .unwrap();
                    match core::write_response(stream, res) {
                        Ok(_) => {}
                        Err(e) => eprintln!("[http reader] Failed to send response {:?}", e),
                    }
                },
            );
        }
    }
}
