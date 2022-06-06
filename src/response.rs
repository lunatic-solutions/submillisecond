use bytes::Bytes;
use http_body::Body;

use crate::{BoxError, Error};

pub use into_response::*;
pub use into_response_parts::*;

mod into_response;
mod into_response_parts;

/// A boxed [`Body`] trait object.
pub type BoxBody = http_body::combinators::UnsyncBoxBody<Bytes, Error>;

/// Type alias for [`http::Response`] whose body defaults to [`Vec<u8>`].
pub type Response<T = BoxBody> = http::Response<T>;

/// Convert a [`http_body::Body`] into a [`BoxBody`].
pub fn boxed<B>(body: B) -> BoxBody
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    try_downcast(body).unwrap_or_else(|body| body.map_err(Error::new).boxed_unsync())
}

pub(crate) fn try_downcast<T, K>(k: K) -> Result<T, K>
where
    T: 'static,
    K: Send + 'static,
{
    let mut k = Some(k);
    if let Some(k) = <dyn std::any::Any>::downcast_mut::<Option<T>>(&mut k) {
        Ok(k.take().unwrap())
    } else {
        Err(k.unwrap())
    }
}
