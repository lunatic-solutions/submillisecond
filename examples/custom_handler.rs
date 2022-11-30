use serde::{Deserialize, Serialize};
use submillisecond::response::IntoResponse;
use submillisecond::{Application, Handler};

#[derive(Clone, Serialize, Deserialize)]
struct Name(String);

impl Handler for Name {
    fn handle(&self, _req: submillisecond::RequestContext) -> submillisecond::response::Response {
        format!("Hello {}!", self.0).into_response()
    }
}

fn main() -> std::io::Result<()> {
    Application::new(|| Name("World".into())).serve("0.0.0.0:3000")
}
