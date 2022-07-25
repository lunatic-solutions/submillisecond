use std::io;

use submillisecond::guard::Guard;
use submillisecond::params::Params;
use submillisecond::{router, Application, RequestContext, Response};

fn global_middleware(req: RequestContext) -> Response {
    println!("[GLOBAL] ENTRY");
    let res = req.next_handler();
    println!("[GLOBAL] EXIT");
    res
}

fn logging_middleware(req: RequestContext) -> Response {
    println!("{} {}", req.method(), req.uri().path());
    let res = req.next_handler();
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
    fn check(&self, _: &RequestContext) -> bool {
        true
    }
}

struct FooGuard;
impl Guard for FooGuard {
    fn check(&self, _: &RequestContext) -> bool {
        true
    }
}

fn main() -> io::Result<()> {
    Application::new(router! {
        with global_middleware;

        "/foo" if FooGuard => {
            with logging_middleware;

            GET "/bar" if BarGuard => foo_bar_handler
        }
        GET "/bar" if BarGuard with logging_middleware => bar_handler
        POST "/foo" with logging_middleware => foo_handler
    })
    .serve("0.0.0.0:3000")
}
