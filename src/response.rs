//! TODO response documentation

use std::borrow::Cow;

/// Type alias for [`http::Response`] whose body defaults to [`Vec<u8>`].
pub type Response<T = Vec<u8>> = http::Response<T>;

/// Converts a type into a [`Response`].
///
/// Types implementing `IntoResponse` can be returned from handlers.
pub trait IntoResponse {
    /// Create a response.
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::default()
    }
}

impl IntoResponse for &[u8] {
    fn into_response(self) -> Response {
        Response::new(self.to_vec())
    }
}

impl IntoResponse for Cow<'static, [u8]> {
    fn into_response(self) -> Response {
        Response::new(self.to_vec())
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        Response::new(self.as_bytes().to_vec())
    }
}

impl IntoResponse for Cow<'static, str> {
    fn into_response(self) -> Response {
        Response::new(self.as_bytes().to_vec())
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        Response::new(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::new(self.into_bytes())
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(value) => value.into_response(),
            Err(err) => err.into_response(),
        }
    }
}

impl IntoResponse for http::StatusCode {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.status_mut() = self;
        res
    }
}

impl IntoResponse for &dyn askama::DynTemplate {
    fn into_response(self) -> Response {
        self.dyn_render().into_response()
    }
}

impl IntoResponse for askama::Error {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}
