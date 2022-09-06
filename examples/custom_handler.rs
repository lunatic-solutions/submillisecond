use serde::{Deserialize, Serialize};
use submillisecond::response::{IntoResponse, Response};
use submillisecond::{Application, Handler, RequestContext};

#[derive(Clone, Serialize, Deserialize)]
struct AppHandler {
    foo: String,
}

impl Handler for AppHandler {
    fn handle(&self, req: RequestContext) -> Response {
        println!("New request: {}", req.uri().path());

        "ok".into_response()
    }
}

fn main() -> std::io::Result<()> {
    Application::new_custom(AppHandler {
        foo: "Hey!".to_string(),
    })
    .serve("0.0.0.0:3000")
}
