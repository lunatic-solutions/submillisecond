use std::{convert, ops};

use crate::core::Body;
use crate::params::Params;
use crate::reader::UriReader;
use crate::Response;

/// Wrapper for [`http::Request`] containing params and cursor.
pub struct RequestContext {
    /// The [`http::Request`] instance.
    pub request: http::Request<Body<'static>>,
    /// Params collected from the router.
    pub params: Params,
    /// The uri reader.
    pub reader: UriReader,
    /// The next handler.
    ///
    /// This is useful for handler layers. See [`RequestContext::next_handler`].
    pub next: Option<fn(RequestContext) -> Response>,
}

impl RequestContext {
    /// Call the next handler, returning the response.
    ///
    /// # Panics
    ///
    /// This function might panic if no next handler exists.
    pub fn next_handler(mut self) -> Response {
        if let Some(next) = self.next.take() {
            next(self)
        } else {
            panic!("no next handler")
        }
    }

    /// Set the next handler.
    ///
    /// This is used internally by the [`router!`](crate::router) macro.
    pub fn set_next_handler(&mut self, next: fn(RequestContext) -> Response) {
        self.next = Some(next);
    }
}

impl<'a> convert::AsRef<http::Request<Body<'a>>> for RequestContext {
    fn as_ref(&self) -> &http::Request<Body<'a>> {
        &self.request
    }
}

impl ops::Deref for RequestContext {
    type Target = http::Request<Body<'static>>;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl ops::DerefMut for RequestContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
}

impl From<http::Request<Body<'static>>> for RequestContext {
    fn from(request: http::Request<Body<'static>>) -> Self {
        let path = request.uri().path().to_string();
        RequestContext {
            request,
            params: Params::default(),
            reader: UriReader::new(path),
            next: None,
        }
    }
}
