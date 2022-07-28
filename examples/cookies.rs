use std::io;

use submillisecond::layers::cookies::{cookies_layer, Cookie, Cookies};
use submillisecond::{router, Application};

fn index(mut cookies: Cookies) -> String {
    let count: i32 = cookies
        .get("count")
        .and_then(|cookie| cookie.value().parse().ok())
        .unwrap_or(0);

    cookies.add(Cookie::new("count", format!("{}", count + 1)));

    count.to_string()
}

fn main() -> io::Result<()> {
    Application::new(router! {
        with cookies_layer;

        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}
