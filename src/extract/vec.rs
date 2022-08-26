use std::convert::Infallible;

use super::FromOwnedRequest;
use crate::RequestContext;

impl FromOwnedRequest for Vec<u8> {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(Vec::from(req.request.into_body().as_slice()))
    }
}
