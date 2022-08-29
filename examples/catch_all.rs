use submillisecond::{router, Application};

fn index() -> &'static str {
    "Hello :)"
}

fn bar() -> &'static str {
    "Foo Bar"
}

fn not_found_foo() -> &'static str {
    "Foo route not found"
}

fn not_found_all() -> &'static str {
    "Route not found"
}

fn main() -> std::io::Result<()> {
    Application::new(router! {
        GET "/" => index
        "/foo" => {
            GET "/bar" => bar
            _ => not_found_foo
        }
        _ => not_found_all
    })
    .serve("0.0.0.0:3000")
}
