use std::io;

use submillisecond::{handler::HandlerFn, router, Application, Middleware};

#[derive(Default)]
struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn before(&mut self, req: &mut submillisecond::Request) {
        println!("[ENTRY] {} {}", req.method(), req.uri().path());
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[EXIT]");
    }
}

#[derive(Default)]
struct FooMiddleware;
impl Middleware for FooMiddleware {
    fn before(&mut self, req: &mut submillisecond::Request) {
        println!("[FOO ENTRY] {} {}", req.method(), req.uri().path());
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[FOO EXIT]");
    }
}

#[derive(Default)]
struct BazMiddleware;
impl Middleware for BazMiddleware {
    fn before(&mut self, req: &mut submillisecond::Request) {
        println!("[BAZ ENTRY] {} {}", req.method(), req.uri().path());
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[BAZ EXIT]");
    }
}

fn foo_bar_handler() -> &'static str {
    "foo bar"
}

fn foo_baz_handler() -> &'static str {
    "foo baz"
}

const FOO_ROUTER: HandlerFn = router! {
    use FooMiddleware;

    "/foo" => {
        GET "/foobar" => foo_bar_handler
    }
};

const BAZ_ROUTER: HandlerFn = router! {
    use BazMiddleware;

    "/baz" => {
        GET "/foobaz" => foo_baz_handler
    }
};

const V1_ROUTER: HandlerFn = router![FOO_ROUTER, BAZ_ROUTER];

fn main() -> io::Result<()> {
    Application::new(router! {
        use LoggingMiddleware;

        "/v1" => V1_ROUTER
        "/v2" => [FOO_ROUTER, BAZ_ROUTER]
    })
    .serve("0.0.0.0:3000")
}
