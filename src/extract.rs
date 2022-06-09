pub mod header_map;
pub mod path;
pub mod query;
pub mod rejection;
pub mod string;
pub mod typed_header;

use crate::{response::IntoResponse, Request};

pub trait FromRequest: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection: IntoResponse;

    /// Perform the extraction.
    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection>;
}
