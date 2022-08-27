use std::convert::Infallible;

use super::FromOwnedRequest;
use crate::core::Body;
use crate::RequestContext;

impl FromOwnedRequest for RequestContext {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req)
    }
}

impl FromOwnedRequest for http::Request<Body<'static>> {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.request)
    }
}
