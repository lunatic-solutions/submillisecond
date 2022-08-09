#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::{io, mem};

pub use http;
use http::{header, HeaderValue};
use lunatic::net::{TcpListener, TcpStream, ToSocketAddrs};
use lunatic::{Mailbox, Process};
pub use submillisecond_macros::*;

pub use crate::error::*;
pub use crate::guard::*;
pub use crate::handler::*;
pub use crate::request::*;
use crate::response::{IntoResponse, Response};

#[macro_use]
pub(crate) mod macros;

#[cfg(feature = "cookie")]
pub mod cookies;
mod core;
pub mod defaults;
pub mod extract;
#[cfg(feature = "json")]
pub mod json;
pub mod params;
pub mod reader;
pub mod response;
#[cfg(feature = "cookie")]
pub mod session;
#[cfg(feature = "template")]
pub mod template;

mod error;
mod guard;
mod handler;
mod request;

/// Signature of router function generated by the [`router!`] macro.
pub type Router = fn(RequestContext) -> Response;

#[derive(Clone, Copy)]
pub struct Application {
    router: Router,
}

impl Application {
    pub fn new(router: Router) -> Self {
        Application { router }
    }

    pub fn serve<A: ToSocketAddrs + Clone>(self, addr: A) -> io::Result<()> {
        #[cfg(not(feature = "logging"))]
        let listener = TcpListener::bind(addr)?;
        #[cfg(feature = "logging")]
        let listener = TcpListener::bind(addr.clone())?;

        #[cfg(feature = "logging")]
        {
            let addrs = addr
                .to_socket_addrs()?
                .map(|addr| {
                    let ip = addr.ip();
                    let ip_string = if ip.is_unspecified() {
                        "localhost".to_string()
                    } else {
                        ip.to_string()
                    };
                    ansi_term::Style::new()
                        .bold()
                        .paint(format!("http://{}:{}", ip_string, addr.port()))
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");
            lunatic_log::info!("Server started on {addrs}");
        }

        while let Ok((stream, _)) = listener.accept() {
            Process::spawn_link(
                (stream, self.router as *const () as usize),
                |(stream, handler_raw): (TcpStream, usize), _: Mailbox<()>| {
                    let handler = unsafe {
                        let pointer = handler_raw as *const ();
                        mem::transmute::<*const (), Router>(pointer)
                    };

                    let request = match core::parse_request(stream.clone()) {
                        Ok(request) => request,
                        Err(err) => {
                            #[allow(unused_variables)]
                            if let Err(err) = core::write_response(stream, err.into_response()) {
                                #[cfg(feature = "logging")]
                                lunatic_log::error!(
                                    "[http reader] Failed to send response {:?}",
                                    err
                                );
                            }
                            return;
                        }
                    };

                    #[cfg(feature = "logging")]
                    {
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

                    let http_version = request.version();

                    let mut response =
                        Handler::handle(&handler, RequestContext::from(request)).into_response();

                    let content_length = response.body().len();
                    *response.version_mut() = http_version;
                    response
                        .headers_mut()
                        .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

                    #[allow(unused_variables)]
                    if let Err(err) = core::write_response(stream, response) {
                        #[cfg(feature = "logging")]
                        lunatic_log::error!("[http reader] Failed to send response {:?}", err);
                    }
                },
            );
        }

        Ok(())
    }
}
