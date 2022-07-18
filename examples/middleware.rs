use std::io;

use submillisecond::{
    guard::Guard, params::Params, router, Application, Middleware, NextFn, Request, Response,
    RouteError,
};

#[derive(Default)]
struct GlobalMiddleware;

impl Middleware for GlobalMiddleware {
    fn apply(&self, req: Request, next: impl NextFn) -> Result<Response, RouteError> {
        println!("[GLOBAL] ENTRY");
        let res = next(req);
        println!("[GLOBAL] EXIT");
        res
    }
}

#[derive(Default)]
struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn apply(&self, req: Request, next: impl NextFn) -> Result<Response, RouteError> {
        println!("{} {}", req.method(), req.uri().path());
        let res = next(req);
        println!("[EXIT]");
        res
    }
}

fn foo_bar_handler() -> &'static str {
    "foo bar"
}

fn foo_handler(params: Params) -> &'static str {
    println!("{params:#?}");
    "foo"
}

fn bar_handler() -> &'static str {
    "bar"
}

struct BarGuard;
impl Guard for BarGuard {
    fn check(&self, _: &Request) -> bool {
        true
    }
}

struct FooGuard;
impl Guard for FooGuard {
    fn check(&self, _: &Request) -> bool {
        true
    }
}

fn main() -> io::Result<()> {
    Application::new(router! {
        use GlobalMiddleware;

        "/foo" if FooGuard use LoggingMiddleware => {
            GET "/bar" if BarGuard => foo_bar_handler
        }
        GET "/bar" if BarGuard use LoggingMiddleware => bar_handler
        POST "/foo" use LoggingMiddleware => foo_handler
    })
    .serve("0.0.0.0:3000")
}
