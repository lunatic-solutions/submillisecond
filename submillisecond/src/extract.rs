//! Types and traits for extracting data from requests.
//!
//! Many of the types and implementations were taken from [Axum](https://crates.io/crates/axum).

pub use path::Path;
#[cfg(feature = "query")]
pub use query::Query;
pub use typed_header::TypedHeader;

pub mod path;
pub mod rejection;

mod header_map;
#[cfg(feature = "json")]
mod json;
mod method;
mod params;
#[cfg(feature = "query")]
mod query;
mod request;
mod route;
mod string;
mod typed_header;
mod vec;

use crate::response::IntoResponse;
use crate::RequestContext;

pub trait FromRequest: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection: IntoResponse;

    /// Perform the extraction.
    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection>;
}

pub trait FromOwnedRequest: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a response.
    type Rejection: IntoResponse;

    /// Extract from an owned instance of the request.
    /// The first extractor in handlers will use this method, and can help avoid
    /// cloning in many cases.
    fn from_owned_request(req: RequestContext) -> Result<Self, Self::Rejection>;
}

impl<T> FromOwnedRequest for T
where
    T: FromRequest,
{
    type Rejection = <T as FromRequest>::Rejection;

    fn from_owned_request(mut req: RequestContext) -> Result<Self, Self::Rejection> {
        T::from_request(&mut req)
    }
}
