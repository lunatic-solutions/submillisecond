use std::io;

use submillisecond::{router, Application};

fn foo_handler() -> &'static str {
    "foo"
}

fn bar_handler() -> &'static str {
    "bar"
}

fn main() -> io::Result<()> {
    // routar();
    let router = router! {
        GET "/foo" if true => foo_handler
        GET "/bar" if true => bar_handler
        POST "/foo" => foo_handler
    };

    Application::new(router).serve("0.0.0.0:3000")
}
