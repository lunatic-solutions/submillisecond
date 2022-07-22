use std::io;

use submillisecond::{
    guard::Guard, params::Params, router, Application, Next, Request, Response, RouteError,
};

fn global_middleware(req: Request, next: impl Next) -> Result<Response, RouteError> {
    println!("[GLOBAL] ENTRY");
    let res = next(req);
    println!("[GLOBAL] EXIT");
    res
}

fn logging_middleware(req: Request, next: impl Next) -> Result<Response, RouteError> {
    println!("{} {}", req.method(), req.uri().path());
    let res = next(req);
    println!("[EXIT]");
    res
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
        use global_middleware;

        "/foo" if FooGuard use logging_middleware => {
            GET "/bar" if BarGuard => foo_bar_handler
        }
        GET "/bar" if BarGuard use logging_middleware => bar_handler
        POST "/foo" use logging_middleware => foo_handler
    })
    .serve("0.0.0.0:3000")
}
