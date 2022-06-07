pub use http::Method;
use lunatic::{
    net::{TcpListener, TcpStream},
    Mailbox, Process,
};
pub use response::Response;
use router::{HandlerFn, Router};
use std::io::Result as IoResult;
pub use submillisecond_macros::*;

pub mod core;
pub mod defaults;
pub mod json;
pub mod response;
pub mod router;

/// Type alias for [`http::Request`] whose body defaults to [`String`].
pub type Request<T = String> = http::Request<T>;

pub struct Application {
    listener: TcpListener,
    router: Router,
}

pub struct ApplicationBuilder {
    router: Router,
}

impl ApplicationBuilder {
    pub fn get(mut self, path: &'static str, handler: HandlerFn) -> ApplicationBuilder {
        self.router.get(path, handler);
        self
    }

    pub fn post(mut self, path: &'static str, handler: HandlerFn) -> ApplicationBuilder {
        self.router.post(path, handler);
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
                |(stream, raw): (TcpStream, Vec<(String, String, usize)>), _: Mailbox<()>| {
                    let router = Router::from_raw(raw);
                    let request = core::parse_request(stream.clone());
                    let matching_handler = router.find_match(&request);
                    let http_version = request.version();
                    let response = matching_handler(request);
                    let res = Response::builder()
                        .version(http_version)
                        .header("content-length", response.body().len())
                        .header("content-type", "text/html")
                        .status(200)
                        .body(response.into_body())
                        .unwrap();
                    match core::write_response(stream, res) {
                        Ok(_) => println!("[http reader] SENT Response 200"),
                        Err(e) => eprintln!("[http reader] Failed to send response {:?}", e),
                    }
                },
            );
        }
    }
}
