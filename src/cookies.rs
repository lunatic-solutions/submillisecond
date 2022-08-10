//! Cookies layer and extractor.

use std::cell::{RefCell, RefMut};
use std::convert::Infallible;
use std::fmt::Write;
use std::ops;

use cookie::CookieJar;
pub use cookie::{Cookie, Key};
use headers::HeaderValue;
use http::header::{COOKIE, SET_COOKIE};
use lunatic::process_local;

use crate::extract::FromRequest;
use crate::response::Response;
use crate::RequestContext;

process_local! {
    /// Process local cookie jar.
    ///
    /// It is advised to use the [`cookies_layer`] and [`Cookies`] extractor to manage this.
    pub static COOKIES: RefCell<CookieJar> = RefCell::new(CookieJar::new());
}

/// Cookies layer which populates the cookie jar from the incoming request,
/// and adds `Set-Cookie` header for modified cookies.
pub fn cookies_layer(req: RequestContext) -> Response {
    // Load cookies from header into cookie jar
    if let Some(cookie_str) = req
        .headers()
        .get(COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
    {
        COOKIES.with_borrow_mut(|mut cookies| {
            for cookie in cookie_str.split(';') {
                if let Ok(cookie) = Cookie::parse_encoded(cookie.to_string()) {
                    cookies.add_original(cookie);
                }
            }
        })
    }

    let mut res = req.next_handler();

    // Push cookies from jar into `Set-Cookie` header, merging if the header is
    // already set
    COOKIES.with_borrow(|cookies| {
        let mut delta = cookies.delta();
        if let Some(first_cookie) = delta.next() {
            let mut header = first_cookie.encoded().to_string();
            for cookie in delta {
                let _ = write!(header, "{};", cookie.encoded());
            }

            if let Ok(header_value) = HeaderValue::from_str(&header) {
                let headers = res.headers_mut();
                match headers.get(SET_COOKIE).and_then(|val| val.to_str().ok()) {
                    // `Set-Cookie` header exists, merge
                    Some(val) => {
                        header.push(';');
                        header.push_str(val);
                        match HeaderValue::from_str(&header) {
                            Ok(header_value) => {
                                headers.insert(SET_COOKIE, header_value);
                            }
                            Err(_) => {
                                headers.insert(SET_COOKIE, header_value);
                            }
                        }
                    }
                    // Insert `Set-Cookie` header
                    None => {
                        headers.insert(SET_COOKIE, header_value);
                    }
                }
            }
        }
    });

    res
}

/// Cookie jar extractor allowing for reading and modifying cookies for a given
/// request.
///
/// The [`cookies_layer`] must be used for this to work.
pub struct Cookies {
    jar: RefMut<'static, CookieJar>,
}

impl ops::Deref for Cookies {
    type Target = CookieJar;

    fn deref(&self) -> &Self::Target {
        &self.jar
    }
}

impl ops::DerefMut for Cookies {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.jar
    }
}

impl FromRequest for Cookies {
    type Rejection = Infallible;

    fn from_request(_req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        let jar = COOKIES.with_borrow_mut(|cookies| cookies);
        Ok(Cookies { jar })
    }
}
