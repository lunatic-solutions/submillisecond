use std::io;

use submillisecond::{guard::Guard, router, Application};

struct ContentLengthGuard(u64);

impl Guard for ContentLengthGuard {
    fn check(&self, req: &submillisecond::Request) -> bool {
        let content_length_header = req
            .headers()
            .get("content-length")
            .and_then(|content_length| content_length.to_str().ok())
            .and_then(|content_length| content_length.parse::<u64>().ok());
        match content_length_header {
            Some(content_length) if content_length == req.body().len() as u64 => {
                self.0 == content_length
            }
            _ => false,
        }
    }
}

fn foo_handler() -> &'static str {
    "foo bar"
}

fn main() -> io::Result<()> {
    Application::new(router! {
        POST "/foo" if ContentLengthGuard(5) || ContentLengthGuard(10) => foo_handler
    })
    .serve("0.0.0.0:3000")
}
