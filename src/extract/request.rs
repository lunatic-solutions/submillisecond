use std::convert::Infallible;

use crate::RequestContext;

use super::FromOwnedRequest;

impl FromOwnedRequest for RequestContext {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req)
    }
}

impl FromOwnedRequest for http::Request<Vec<u8>> {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.request)
    }
}
