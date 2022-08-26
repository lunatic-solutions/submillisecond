use std::io;
use std::time::Duration;

use submillisecond::{router, Application};

// Each request has a default 5 minute timeout.
// Waiting for 10 minutes should fail.
fn index() {
    lunatic::sleep(Duration::from_secs(60 * 10));
}

fn main() -> io::Result<()> {
    Application::new(router! {
        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}
