use std::{convert::Infallible, mem};

use crate::Request;

use super::FromRequest;

impl FromRequest for Vec<u8> {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        let body = mem::take(req.body_mut());
        Ok(body)
    }

    fn from_owned_request(req: Request) -> Result<Self, Self::Rejection> {
        Ok(req.request.into_body())
    }
}
