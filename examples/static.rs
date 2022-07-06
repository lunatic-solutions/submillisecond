use std::io;

use submillisecond::{router, static_dir, Application};

fn main() -> io::Result<()> {
    Application::new(router! {
        "/" => static_dir!("./static")
    })
    .serve("0.0.0.0:3000")
}
