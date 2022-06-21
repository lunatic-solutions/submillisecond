use std::io;

use submillisecond::{router, Application, static_dir};

fn main() -> io::Result<()> {
    Application::new(router! {
        "/" => static_dir!("./static")
    })
    .serve("0.0.0.0:3000")
}
