use std::io;

use submillisecond::{router, Application, Middleware};

struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn before(req: &mut submillisecond::Request) -> Self {
        println!("{} {}", req.method(), req.uri().path());

        LoggingMiddleware
    }

    fn after(self, _res: &mut submillisecond::Response) {
        println!("[EXIT]");
    }
}

fn foo_bar_handler() -> &'static str {
    "foo bar"
}

fn main() -> io::Result<()> {
    Application::new(router! {
        use LoggingMiddleware;

        "/foo" => {
            GET "/bar" => foo_bar_handler
        }
    })
    .serve("0.0.0.0:3000")
}

fn __router(
    mut __req: ::submillisecond::Request,
) -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
    let mut params = ::submillisecond::params::Params::new();
    let __middleware_calls = (<LoggingMiddleware as ::submillisecond::Middleware>::before(
        &mut __req,
    ),);
    let mut __resp = match *__req.method() {
        ::http::Method::GET => {
            let path = __req.uri().path().to_string();
            let mut reader = ::submillisecond::core::UriReader::new(path);
            if reader.peek(8usize) == "/foo/bar" {
                reader.read(8usize);
                if reader.is_empty() {
                    ::submillisecond::Application::merge_extensions(&mut __req, &mut params);
                    let mut __resp = ::submillisecond::response::IntoResponse::into_response(
                        ::submillisecond::handler::Handler::handle(
                            foo_bar_handler
                                as ::submillisecond::handler::FnPtr<
                                    _,
                                    _,
                                    { ::submillisecond::handler::arity(&foo_bar_handler) },
                                >,
                            __req,
                        ),
                    );
                    return ::std::result::Result::Ok(__resp);
                }
            }
            return ::std::result::Result::Err(
                ::submillisecond::router::RouteError::RouteNotMatch(__req),
            );
        }
        ::http::Method::POST => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
        ::http::Method::PUT => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
        ::http::Method::DELETE => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
        ::http::Method::HEAD => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
        ::http::Method::OPTIONS => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
        ::http::Method::PATCH => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
        _ => ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
    };
    if let Ok(ref mut __resp) = &mut __resp {
        ::submillisecond::Middleware::after(__middleware_calls.0, __resp);
    }
    __resp
}
