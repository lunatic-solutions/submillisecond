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
