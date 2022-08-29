use submillisecond::params::Params;
use submillisecond::response::Response;
use submillisecond::{router, Application, Guard, Handler, RequestContext};

fn global_middleware(req: RequestContext) -> Response {
    println!("[GLOBAL] ENTRY");
    let res = req.next_handler();
    println!("[GLOBAL] EXIT");
    res
}

struct LoggingMiddleware {
    level: u8,
}

impl LoggingMiddleware {
    const fn new(level: u8) -> Self {
        LoggingMiddleware { level }
    }
}

impl Handler for LoggingMiddleware {
    fn handle(&self, req: RequestContext) -> Response {
        if self.level == 0 {
            return req.next_handler();
        }

        println!("{} {}", req.method(), req.uri().path());
        let res = req.next_handler();
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

fn main() -> std::io::Result<()> {
    const LOGGER: LoggingMiddleware = LoggingMiddleware::new(1);

    Application::new(router! {
        with global_middleware;

        "/foo" if FooGuard => {
            with LOGGER;

            GET "/bar" if BarGuard => foo_bar_handler
        }
        GET "/bar" if BarGuard with LOGGER => bar_handler
        POST "/foo" with LOGGER => foo_handler
    })
    .serve("0.0.0.0:3000")
}
