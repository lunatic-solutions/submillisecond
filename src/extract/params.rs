use std::convert::Infallible;

use submillisecond_core::params::Params;

use super::FromRequest;

impl FromRequest for Params {
    type Rejection = Infallible;

    fn from_request(req: &mut crate::Request) -> Result<Self, Self::Rejection> {
        Ok(req.extensions().get::<Params>().unwrap().clone())
    }
}
