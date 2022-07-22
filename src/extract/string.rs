use std::mem;

use crate::Request;

use super::{
    rejection::{InvalidUtf8, StringRejection},
    FromRequest,
};

impl FromRequest for String {
    type Rejection = StringRejection;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        let body = mem::take(req.body_mut());
        let string = String::from_utf8(body).map_err(InvalidUtf8::from_err)?;
        Ok(string)
    }

    fn from_owned_request(req: Request) -> Result<Self, Self::Rejection> {
        Ok(std::str::from_utf8(req.body())
            .map_err(InvalidUtf8::from_err)?
            .to_string())
    }
}
