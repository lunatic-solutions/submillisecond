use std::convert::Infallible;

use http::Method;

use crate::RequestContext;

use super::FromRequest;

impl FromRequest for Method {
    type Rejection = Infallible;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        Ok(req.method().clone())
    }
}
