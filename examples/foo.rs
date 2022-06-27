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
struct LoggingMiddleware {
    request_id: String,
}

impl Middleware for LoggingMiddleware {
    fn before(&mut self, req: &mut submillisecond::Request) {
        self.request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|req_id| req_id.to_str().ok())
            .map(|req_id| req_id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("[ENTER] request {}", self.request_id);
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[EXIT] request {}", self.request_id);
    }
}

fn foo_handler(params: Params) -> &'static str {
    println!("{params:#?}");
    "foo"
}

fn bar_handler() -> &'static str {
    "bar"
}

struct FakeGuard;

impl Guard for FakeGuard {
    fn check(&self, _: &submillisecond::Request) -> bool {
        true
    }
}

struct BarGuard;
impl Guard for BarGuard {
    fn check(&self, _: &submillisecond::Request) -> bool {
        true
    }
}

fn main() -> io::Result<()> {
    Application::new(router! {
        use GlobalMiddleware;

        "/foo" if FakeGuard => {
            GET "/bar" if BarGuard use LoggingMiddleware => foo_handler
        }
        GET "/bar" if BarGuard => bar_handler
        POST "/foo" => foo_handler
    })
    .serve("0.0.0.0:3000")
}
