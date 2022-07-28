use std::convert::Infallible;

use http::Method;

use super::FromRequest;
use crate::RequestContext;

impl FromRequest for Method {
    type Rejection = Infallible;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.method().clone())
    }
}
