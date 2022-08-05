use std::convert::Infallible;

use super::FromRequest;

/// Extract the remainder of the url from a wildcard route.
///
/// # Example
///
/// ```
/// fn foo_handler(Splat(splat): Splat) {
///     // GET "/foo-bar" prints "bar"
///     println!("{splat}");
/// }
///
/// router! {
///     GET "/foo-*" => foo_handler
/// }
/// ```
pub struct Splat(pub String);

impl FromRequest for Splat {
    type Rejection = Infallible;

    fn from_request(req: &mut crate::RequestContext) -> Result<Self, Self::Rejection> {
        Ok(Splat(req.reader.uri[req.reader.cursor..].to_string()))
    }
}
