use http::HeaderMap;
use submillisecond::{route, Application, Request};

#[route("/hey")]
fn hello(req: Request, headers: HeaderMap) -> String {
    println!("{:#?}", headers);
    println!("{:#?}", req.uri());

    "Hello world!".to_string()
}

fn main() {
    Application::build()
        .get("/hello", hello)
        .listen(3000)
        .unwrap()
        .start_server();
}
