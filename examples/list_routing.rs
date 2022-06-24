use std::io;

use submillisecond::{handler::HandlerFn, router, Application, Middleware};

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

fn foo_baz_handler() -> &'static str {
    "foo baz"
}

const FOO_ROUTER: HandlerFn = router! {
    use LoggingMiddleware;

    "/foo" => {
        GET "/foobar" => foo_bar_handler
    }
};

const BAZ_ROUTER: HandlerFn = router! {
    use LoggingMiddleware;

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
