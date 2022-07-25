use std::io;

use submillisecond::{
    extract::Path, guard::Guard, params::Params, router, Application, RequestContext, Response,
};

fn logging_middleware(req: RequestContext) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|req_id| req_id.to_str().ok())
        .map(|req_id| req_id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("[ENTER] request {request_id}");
    let res = req.next_handler();
    println!("[EXIT] request {request_id}");
    res
}

fn foo_handler(params: Params) -> &'static str {
    println!("{params:#?}");
    "foo"
}

fn bar_handler(Path((a, b, c)): Path<(String, String, String)>) -> &'static str {
    println!("GOT PATH {:?} {:?} {:?}", a, b, c);
    "bar"
}

struct FakeGuard;

impl Guard for FakeGuard {
    fn check(&self, _: &submillisecond::RequestContext) -> bool {
        true
    }
}

struct BarGuard;
impl Guard for BarGuard {
    fn check(&self, _: &submillisecond::RequestContext) -> bool {
        true
    }
}

fn main() -> io::Result<()> {
    Application::new(router! {
        "/:a" if FakeGuard => {
            "/:b" => {
                GET "/:c" if BarGuard with logging_middleware => bar_handler
            }
        }
        GET "/hello/:x/:y/:z" if BarGuard => foo_handler
    })
    .serve("0.0.0.0:3000")
}
