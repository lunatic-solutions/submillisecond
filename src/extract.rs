//! Types and traits for extracting data from requests.
//!
//! Many of the types and implementations were taken from [Axum](https://crates.io/crates/axum).

pub use path::Path;
pub use query::Query;
pub use typed_header::TypedHeader;

mod header_map;
mod json;
pub mod path;
mod query;
pub mod rejection;
mod string;
mod typed_header;
mod vec;

use crate::{response::IntoResponse, Request};

pub trait FromRequest: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection: IntoResponse;

    /// Perform the extraction.
    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection>;
}
