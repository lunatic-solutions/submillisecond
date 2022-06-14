use std::convert::Infallible;

use http::HeaderMap;

use crate::Request;

use super::FromRequest;

impl FromRequest for HeaderMap {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        Ok(req.headers().clone())
    }
}
