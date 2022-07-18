use std::io;

use submillisecond::{
    router, Application, Middleware, NextFn, Request, Response, RouteError, Router,
};

#[derive(Default)]
struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn apply(&self, req: Request, next: impl NextFn) -> Result<Response, RouteError> {
        println!("[ENTRY] {} {}", req.method(), req.uri().path());
        let res = next(req);
        println!("[EXIT]");
        res
    }
}

#[derive(Default)]
struct FooMiddleware;
impl Middleware for FooMiddleware {
    fn apply(&self, req: Request, next: impl NextFn) -> Result<Response, RouteError> {
        println!("[FOO ENTRY] {} {}", req.method(), req.uri().path());
        let res = next(req);
        println!("[FOO EXIT]");
        res
    }
}

#[derive(Default)]
struct BazMiddleware;
impl Middleware for BazMiddleware {
    fn apply(&self, req: Request, next: impl NextFn) -> Result<Response, RouteError> {
        println!("[BAZ ENTRY] {} {}", req.method(), req.uri().path());
        let res = next(req);
        println!("[BAZ EXIT]");
        res
    }
}

fn foo_bar_handler() -> &'static str {
    "foo bar"
}

fn foo_baz_handler() -> &'static str {
    "foo baz"
}

const FOO_ROUTER: Router = router! {
    use FooMiddleware;

    "/foo" => {
        GET "/foobar" => foo_bar_handler
    }
};

const BAZ_ROUTER: Router = router! {
    use BazMiddleware;

    "/baz" => {
        GET "/foobaz" => foo_baz_handler
    }
};

const V1_ROUTER: Router = router![FOO_ROUTER, BAZ_ROUTER];

fn main() -> io::Result<()> {
    Application::new(router! {
        use LoggingMiddleware;

        "/v1" => V1_ROUTER
        "/v2" => [FOO_ROUTER, BAZ_ROUTER]
    })
    .serve("0.0.0.0:3000")
}
