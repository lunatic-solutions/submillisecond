use std::io;

use handler::HandlerFn;
pub use http;
use lunatic::{
    net::{TcpListener, ToSocketAddrs},
    process::StartProcess,
};
use params::Params;
pub use submillisecond_macros::*;
use supervisor::RequestSupervisorProcess;

pub use crate::error::{BoxError, Error};
pub use crate::response::Response;

#[macro_use]
pub(crate) mod macros;

pub mod core;
pub mod defaults;
mod error;
pub mod extract;
pub mod guard;
pub mod handler;
pub mod json;
pub mod params;
pub mod request_context;
pub mod response;
pub mod router;
pub mod supervisor;
pub mod template;

/// Type alias for [`http::Request`] whose body defaults to [`String`].
pub type Request<T = Vec<u8>> = http::Request<T>;

#[derive(Clone, Copy)]
pub struct Application {
    router: HandlerFn,
}

impl Application {
    pub fn new(router: HandlerFn) -> Self {
        Application { router }
    }

    pub fn merge_extensions(request: &mut Request, params: &mut Params) {
        let extensions = request.extensions_mut();
        match extensions.get_mut::<Params>() {
            Some(ext_params) => {
                ext_params.merge(params.clone());
            }
            None => {
                extensions.insert(params.clone());
            }
        };
    }

    pub fn serve<A: ToSocketAddrs>(self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;

        while let Ok((stream, _)) = listener.accept() {
            RequestSupervisorProcess::start((stream, self.router as *const () as usize), None);
        }

        Ok(())
    }
}

pub trait Middleware {
    fn before(&mut self, req: &mut Request);
    fn after(&self, res: &mut Response);
}
