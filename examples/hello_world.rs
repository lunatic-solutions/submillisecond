use submillisecond::{router, Application};

fn index() -> &'static str {
    "Hello :)"
}

fn main() -> std::io::Result<()> {
    Application::new(router! {
        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}
