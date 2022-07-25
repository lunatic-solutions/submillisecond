use std::convert::Infallible;

use crate::{params::Params, RequestContext};

use super::FromOwnedRequest;

impl FromOwnedRequest for Params {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.params)
    }
}
