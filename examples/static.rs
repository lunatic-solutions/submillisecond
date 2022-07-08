use std::io;

use submillisecond::{static_router, Application};

fn main() -> io::Result<()> {
    Application::new(static_router!("./static")).serve("0.0.0.0:3000")
}
