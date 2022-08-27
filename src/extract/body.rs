use std::convert::Infallible;

use super::FromOwnedRequest;
use crate::{Body, RequestContext};

impl FromOwnedRequest for Body<'static> {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(*req.body())
    }
}
