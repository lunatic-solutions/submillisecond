use std::io;

use submillisecond::{router, Application};
use submillisecond_core::router::params::Params;

fn foo_handler(params: Params) -> &'static str {
    println!("{params:#?}");
    "foo"
}

fn bar_handler() -> &'static str {
    "bar"
}

fn main() -> io::Result<()> {
    Application::new(router! {
        "/foo" if true => {
            GET "/bar" => foo_handler
        }
        GET "/bar" if true => bar_handler
        POST "/foo" => foo_handler
    })
    .serve("0.0.0.0:3000")
}
