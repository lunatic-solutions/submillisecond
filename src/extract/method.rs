use std::convert::Infallible;

use http::Method;

use super::FromRequest;

impl FromRequest for Method {
    type Rejection = Infallible;

    fn from_request(req: &mut crate::Request) -> Result<Self, Self::Rejection> {
        Ok(req.method().clone())
    }
}
