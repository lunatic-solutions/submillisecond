use std::io;

use cookie::Cookie;
use submillisecond::cookies::{cookies_layer, Cookies, Key};
use submillisecond::session::{init_session, Session};
use submillisecond::{router, Application};

fn index(mut cookies: Cookies) -> String {
    let count: i32 = cookies
        .get("count")
        .and_then(|cookie| cookie.value().parse().ok())
        .unwrap_or(0);

    cookies.add(Cookie::new("count", format!("{}", count + 1)));

    count.to_string()
}

fn session(mut session: Session<i32>) -> String {
    if *session < 10 {
        *session += 1;
    }
    session.to_string()
}

fn session_bool(mut session: Session<bool>) -> String {
    *session = !*session;
    session.to_string()
}

fn main() -> io::Result<()> {
    init_session(Key::from(&[2; 64]));

    Application::new(router! {
        with cookies_layer;

        GET "/" => index
        GET "/session" => session
        GET "/session-bool" => session_bool
    })
    .serve("0.0.0.0:3000")
}
