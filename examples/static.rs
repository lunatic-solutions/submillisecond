use std::io;

use submillisecond::{response::IntoResponse, router, static_dir, Application};

fn main() -> io::Result<()> {
    Application::new(router! {
        "/" => static_dir!("./static")
    })
    .serve("0.0.0.0:3000")
}
