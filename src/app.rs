use std::io;
use std::marker::PhantomData;

pub use http;
use lunatic::net::{TcpListener, ToSocketAddrs};
use lunatic::Process;

use crate::supervisor::request_supervisor;
use crate::ProcessSafeHandler;

/// An application containing a router for listening and handling incoming
/// requests.
///
/// # Example
///
/// ```
/// use submillisecond::{router, Application};
///
/// fn index() -> &'static str { "Welcome" }
///
/// Application::new(router! {
///     GET "/" => index
/// })
/// .serve("0.0.0.0:3000")
/// ```
#[derive(Clone, Copy)]
pub struct Application<T, Kind, Arg, Ret> {
    handler: T,
    phantom: PhantomData<(Kind, Arg, Ret)>,
}

impl<T, Kind, Arg, Ret> Application<T, Kind, Arg, Ret>
where
    T: ProcessSafeHandler<Kind, Arg, Ret>,
{
    /// Creates a new application with a given router.
    pub fn new(handler: fn() -> T) -> Self {
        Application {
            handler: handler(),
            phantom: PhantomData,
        }
    }

    /// Listen on `addr` to receive incoming requests, and handling them with
    /// the router.
    pub fn serve<A>(self, addr: A) -> io::Result<()>
    where
        A: ToSocketAddrs + Clone,
    {
        let safe_handler = self.handler.safe_handler();
        let listener = TcpListener::bind(addr.clone())?;
        log_server_start(addr);
        while let Ok((stream, _)) = listener.accept() {
            Process::spawn_link((stream, safe_handler.clone()), request_supervisor);
        }

        Ok(())
    }
}

#[cfg(feature = "logging")]
fn log_server_start<A: ToSocketAddrs>(addr: A) {
    use lunatic_log::subscriber::fmt::FmtSubscriber;
    use lunatic_log::{LevelFilter, __lookup_logging_process, info};

    // If no logging process is running, start the default logger.
    if __lookup_logging_process().is_none() {
        lunatic_log::init(
            FmtSubscriber::new(LevelFilter::Trace)
                .with_color(true)
                .with_level(true)
                .with_target(true),
        );
    }
    // Make address bold.
    let addrs = addr
        .to_socket_addrs()
        .unwrap() // is ok if the code is log is executed after bind
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
    info!("Server started on {addrs}");
}

#[cfg(not(feature = "logging"))]
fn log_server_start<A: ToSocketAddrs>(_addr: A) {}
