use std::{convert, ops};

use crate::{core::UriReader, params::Params};

/// Wrapper for [`http::Request`] containing params and cursor.
#[derive(Debug)]
pub struct Request {
    pub request: http::Request<Vec<u8>>,
    pub params: Params,
    pub reader: UriReader,
}

impl convert::AsRef<http::Request<Vec<u8>>> for Request {
    fn as_ref(&self) -> &http::Request<Vec<u8>> {
        &self.request
    }
}

impl ops::Deref for Request {
    type Target = http::Request<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl ops::DerefMut for Request {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
}

impl From<http::Request<Vec<u8>>> for Request {
    fn from(request: http::Request<Vec<u8>>) -> Self {
        let path = request.uri().path().to_string();
        Request {
            request,
            params: Params::default(),
            reader: UriReader::new(path),
        }
    }
}
