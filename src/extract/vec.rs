use std::convert::Infallible;

use crate::RequestContext;

use super::FromOwnedRequest;

impl FromOwnedRequest for Vec<u8> {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.request.into_body())
    }
}
