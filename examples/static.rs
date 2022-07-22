use std::io;

use http::StatusCode;
use submillisecond::{static_router, Application};

fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Resource not found")
}

fn main() -> io::Result<()> {
    Application::new(static_router!("./static", handle_404)).serve("0.0.0.0:3000")
}
