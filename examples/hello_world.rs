use submillisecond::{route, Application};

#[route("/hey")]
fn hello() -> String {
    "Hello world!".to_string()
}

fn main() {
    Application::build()
        .get("/hello", hello)
        .listen(3000)
        .unwrap()
        .start_server();
}
