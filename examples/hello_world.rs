use submillisecond::{get, Application, Request};
use submillisecond_core::router::params::Params;

#[get("/")]
fn root() -> &'static str {
    "OK"
}

#[get("/users/:id")]
fn hello(req: Request) -> String {
    let params: &Params = req.extensions().get().unwrap();
    for param in params.iter() {
        dbg!(param);
    }

    let id = params.get("id").unwrap();
    format!("Welcome, {id}")
}

fn main() {
    Application::build()
        .route(root)
        .route(hello)
        .listen(3000)
        .unwrap()
        .start_server();
}
