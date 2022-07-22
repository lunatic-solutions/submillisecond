use std::io;

use submillisecond::{router, Application, IntoResponse, Next, Request, Router};

fn logging_middleware(req: Request, next: impl Next) -> impl IntoResponse {
    println!("[ENTRY] {} {}", req.method(), req.uri().path());
    let res = next(req);
    println!("[EXIT]");
    res
}

fn foo_middleware(req: Request, next: impl Next) -> impl IntoResponse {
    println!("[FOO ENTRY] {} {}", req.method(), req.uri().path());
    let res = next(req);
    println!("[FOO EXIT]");
    res
}

fn baz_middleware(req: Request, next: impl Next) -> impl IntoResponse {
    println!("[BAZ ENTRY] {} {}", req.method(), req.uri().path());
    let res = next(req);
    println!("[BAZ EXIT]");
    res
}

fn foo_bar_handler() -> &'static str {
    "foo bar"
}

fn foo_baz_handler() -> &'static str {
    "foo baz"
}

const FOO_ROUTER: Router = router! {
    use foo_middleware;

    "/foo" => {
        GET "/foobar" => foo_bar_handler
    }
};

const BAZ_ROUTER: Router = router! {
    use baz_middleware;

    "/baz" => {
        GET "/foobaz" => foo_baz_handler
    }
};

const V1_ROUTER: Router = router![FOO_ROUTER, BAZ_ROUTER];

fn main() -> io::Result<()> {
    Application::new(router! {
        use logging_middleware;

        "/v1" => V1_ROUTER
        "/v2" => [FOO_ROUTER, BAZ_ROUTER]
    })
    .serve("0.0.0.0:3000")
}
