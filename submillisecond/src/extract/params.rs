use std::convert::Infallible;

use super::FromOwnedRequest;
use crate::params::Params;
use crate::RequestContext;

impl FromOwnedRequest for Params {
    type Rejection = Infallible;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.params)
    }
}
