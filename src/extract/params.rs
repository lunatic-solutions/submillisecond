use std::convert::Infallible;

use crate::{params::Params, Request};

use super::FromRequest;

impl FromRequest for Params {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        Ok(req.params.clone())
    }
}
