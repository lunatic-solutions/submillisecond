use std::io;

use submillisecond::{router, Application, Middleware};
use submillisecond_core::router::params::Params;

struct LoggingMiddleware {
    request_id: String,
}

impl Middleware for LoggingMiddleware {
    fn before(req: &mut submillisecond::Request) -> Self {
        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|req_id| req_id.to_str().ok())
            .map(|req_id| req_id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("[ENTER] request {}", request_id);
        LoggingMiddleware { request_id }
    }

    fn after(self, _res: &mut submillisecond::Response) {
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

fn main() -> io::Result<()> {
    // Application::new(router! {
    //     "/foo" if true => {
    //         GET "/bar" use LoggingMiddleware => foo_handler
    //     }
    //     GET "/bar" if true => bar_handler
    //     POST "/foo" => foo_handler
    // })
    // .serve("0.0.0.0:3000")
    Ok(())
}
