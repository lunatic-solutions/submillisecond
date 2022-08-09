use std::io;

use lunatic_log::subscriber::fmt::FmtSubscriber;
use lunatic_log::LevelFilter;
use submillisecond::{router, Application};

fn index() -> &'static str {
    "Hello :)"
}

fn main() -> io::Result<()> {
    lunatic_log::init(
        FmtSubscriber::new(LevelFilter::Trace)
            .with_color(true)
            .with_level(true)
            .with_target(true),
    );

    Application::new(router! {
        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}
