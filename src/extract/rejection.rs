use std::fmt;

use http::StatusCode;

use crate::{response::IntoResponse, BoxError, Error, Response};

use super::path::FailedToDeserializePathParams;

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "No paths parameters found for matched route. Are you also extracting `Request<_>`?"]
    /// Rejection type used if axum's internal representation of path parameters
    /// is missing. This is commonly caused by extracting `Request<_>`. `Path`
    /// must be extracted first.
    pub struct MissingPathParams;
}

composite_rejection! {
    /// Rejection used for [`Path`](super::Path).
    ///
    /// Contains one variant for each way the [`Path`](super::Path) extractor
    /// can fail.
    pub enum PathRejection {
        FailedToDeserializePathParams,
        MissingPathParams,
    }
}

/// Rejection type for extractors that deserialize query strings if the input
/// couldn't be deserialized into the target type.
#[derive(Debug)]
pub struct FailedToDeserializeQueryString {
    error: Error,
    type_name: &'static str,
}

impl FailedToDeserializeQueryString {
    #[doc(hidden)]
    pub fn __private_new<T, E>(error: E) -> Self
    where
        E: Into<BoxError>,
    {
        FailedToDeserializeQueryString {
            error: Error::new(error),
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl IntoResponse for FailedToDeserializeQueryString {
    fn into_response(self) -> Response {
        (http::StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
    }
}

impl std::fmt::Display for FailedToDeserializeQueryString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to deserialize query string. Expected something of type `{}`. Error: {}",
            self.type_name, self.error,
        )
    }
}

impl std::error::Error for FailedToDeserializeQueryString {}

composite_rejection! {
    /// Rejection used for [`Query`](super::Query).
    ///
    /// Contains one variant for each way the [`Query`](super::Query) extractor
    /// can fail.
    pub enum QueryRejection {
        FailedToDeserializeQueryString,
    }
}

/// Rejection type used if you try and extract the request body more than
/// once.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct BodyAlreadyExtracted;

impl BodyAlreadyExtracted {
    const BODY: &'static str = "Cannot have two request body extractors for a single handler";
}

impl IntoResponse for BodyAlreadyExtracted {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Self::BODY).into_response()
    }
}

impl fmt::Display for BodyAlreadyExtracted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self::BODY)
    }
}

impl std::error::Error for BodyAlreadyExtracted {}

define_rejection! {
    #[status = PAYLOAD_TOO_LARGE]
    #[body = "Failed to buffer the request body"]
    /// Encountered some other error when buffering the body.
    ///
    /// This can  _only_ happen when you're using [`tower_http::limit::RequestBodyLimitLayer`] or
    /// otherwise wrapping request bodies in [`http_body::Limited`].
    ///
    /// [`tower_http::limit::RequestBodyLimitLayer`]: https://docs.rs/tower-http/0.3/tower_http/limit/struct.RequestBodyLimitLayer.html
    pub struct LengthLimitError(Error);
}

define_rejection! {
    #[status = BAD_REQUEST]
    #[body = "Failed to buffer the request body"]
    /// Encountered an unknown error when buffering the body.
    pub struct UnknownBodyError(Error);
}

define_rejection! {
    #[status = BAD_REQUEST]
    #[body = "Request body didn't contain valid UTF-8"]
    /// Rejection type used when buffering the request into a [`String`] if the
    /// body doesn't contain valid UTF-8.
    pub struct InvalidUtf8(Error);
}

composite_rejection! {
    /// Rejection used for [`String`].
    ///
    /// Contains one variant for each way the [`String`] extractor can fail.
    pub enum StringRejection {
        BodyAlreadyExtracted,
        // FailedToBufferBody,
        InvalidUtf8,
    }
}
