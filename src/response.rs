pub use into_response::*;
pub use into_response_parts::*;

mod into_response;
mod into_response_parts;

/// Type alias for [`http::Response`] whose body defaults to [`Vec<u8>`].
pub type Response<T = Vec<u8>> = http::Response<T>;
