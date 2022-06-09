use submillisecond::{extract::path::Path, get, Application};

#[get("/users/:id")]
fn hello(Path(id): Path<String>) -> String {
    format!("Welcome, {id}")
}

fn main() {
    Application::build()
        .route(hello)
        .listen(3000)
        .unwrap()
        .start_server();
}
