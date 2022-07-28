use super::rejection::{InvalidUtf8, StringRejection};
use super::FromOwnedRequest;
use crate::RequestContext;

impl FromOwnedRequest for String {
    type Rejection = StringRejection;

    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection> {
        Ok(String::from_utf8(req.request.into_body()).map_err(InvalidUtf8::from_err)?)
    }
}
