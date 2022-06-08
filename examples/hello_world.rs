use http::HeaderMap;
use submillisecond::{get, Application, Request};

#[get("/hey")]
fn hello(req: Request, headers: HeaderMap) -> String {
    println!("{:#?}", headers);
    println!("{:#?}", req.uri());

    "Hello world!".to_string()
}

fn main() {
    Application::build()
        .route(hello)
        .listen(3000)
        .unwrap()
        .start_server();
}
