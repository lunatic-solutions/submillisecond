use std::{convert, ops};

use crate::core::UriReader;
use crate::params::Params;
use crate::Response;

/// Wrapper for [`http::Request`] containing params and cursor.
pub struct RequestContext {
    pub request: http::Request<Vec<u8>>,
    pub params: Params,
    pub reader: UriReader,
    pub next: Option<fn(RequestContext) -> Response>,
}

impl RequestContext {
    pub fn next_handler(mut self) -> Response {
        if let Some(next) = self.next.take() {
            next(self)
        } else {
            panic!("no next handler")
        }
    }

    pub fn set_next_handler(&mut self, next: fn(RequestContext) -> Response) {
        self.next = Some(next);
    }
}

impl convert::AsRef<http::Request<Vec<u8>>> for RequestContext {
    fn as_ref(&self) -> &http::Request<Vec<u8>> {
        &self.request
    }
}

impl ops::Deref for RequestContext {
    type Target = http::Request<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl ops::DerefMut for RequestContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
}

impl From<http::Request<Vec<u8>>> for RequestContext {
    fn from(request: http::Request<Vec<u8>>) -> Self {
        let path = request.uri().path().to_string();
        RequestContext {
            request,
            params: Params::default(),
            reader: UriReader::new(path),
            next: None,
        }
    }
}
