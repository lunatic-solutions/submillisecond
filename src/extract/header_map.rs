use std::convert::Infallible;

use http::HeaderMap;

use crate::RequestContext;

use super::FromRequest;

impl FromRequest for HeaderMap {
    type Rejection = Infallible;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.headers().clone())
    }
}
