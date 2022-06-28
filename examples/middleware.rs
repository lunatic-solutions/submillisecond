use std::io;

use submillisecond::{guard::Guard, params::Params, router, Application, Middleware};

#[derive(Default)]
struct GlobalMiddleware;

impl Middleware for GlobalMiddleware {
    fn before(&mut self, _req: &mut submillisecond::Request) {
        println!("[GLOBAL] ENTRY");
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[GLOBAL] EXIT");
    }
}

#[derive(Default)]
struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn before(&mut self, req: &mut submillisecond::Request) {
        println!("{} {}", req.method(), req.uri().path());

        // LoggingMiddleware
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[EXIT]");
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
    fn check(&self, _: &submillisecond::Request) -> bool {
        true
    }
}

struct FooGuard;
impl Guard for FooGuard {
    fn check(&self, _: &submillisecond::Request) -> bool {
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
