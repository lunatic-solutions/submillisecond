use submillisecond::{Application, Request, Response};

fn hello(_: Request<String>) -> Response<String> {
    let body = String::from("Hello world!");
    Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("Content-Length", body.len())
        .body(body)
        .unwrap()
}

fn main() {
    Application::build()
        .get("/hello", hello)
        .listen(3000)
        .unwrap()
        .start_server();
}
