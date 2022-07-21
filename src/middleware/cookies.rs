use std::{cell::RefCell, convert::Infallible};

use crate::{extract::FromRequest, process_local, Next, Request, Response, RouteError};

process_local! {
    static COOKIES: RefCell<Option<CookieState>> = RefCell::new(None);
}

pub fn cookies(req: Request, next: impl Next) -> Result<Response, RouteError> {
    COOKIES.set(Some(CookieState {}));
    next(req)
}

pub struct Cookies {}

impl FromRequest for Cookies {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        let p = COOKIES.with_borrow(|cookies| cookies);
        todo!()
    }
}

struct CookieState {}
