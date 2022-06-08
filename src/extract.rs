pub mod path;

use std::convert::Infallible;

use http::HeaderMap;

use crate::{response::IntoResponse, Request};

pub trait FromRequest: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection: IntoResponse;

    /// Perform the extraction.
    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection>;
}

impl FromRequest for HeaderMap {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        Ok(req.headers().clone())
    }
}
