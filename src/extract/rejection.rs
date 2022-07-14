use crate::{response::IntoResponse, Response, RouteError};
#[cfg(feature = "query")]
use crate::{BoxError, Error};

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
#[cfg(feature = "query")]
#[derive(Debug)]
pub struct FailedToDeserializeQueryString {
    error: Error,
    type_name: &'static str,
}

#[cfg(feature = "query")]
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

#[cfg(feature = "query")]
impl IntoResponse for FailedToDeserializeQueryString {
    fn into_response(self) -> Result<Response, RouteError> {
        (http::StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
    }
}

#[cfg(feature = "query")]
impl std::fmt::Display for FailedToDeserializeQueryString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to deserialize query string. Expected something of type `{}`. Error: {}",
            self.type_name, self.error,
        )
    }
}

#[cfg(feature = "query")]
impl std::error::Error for FailedToDeserializeQueryString {}

#[cfg(feature = "query")]
composite_rejection! {
    /// Rejection used for [`Query`](super::Query).
    ///
    /// Contains one variant for each way the [`Query`](super::Query) extractor
    /// can fail.
    pub enum QueryRejection {
        FailedToDeserializeQueryString,
    }
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
        InvalidUtf8,
    }
}

#[cfg(feature = "json")]
define_rejection! {
    #[status = UNPROCESSABLE_ENTITY]
    #[body = "Failed to deserialize the JSON body into the target type"]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    /// Rejection type for [`Json`](crate::json::Json).
    ///
    /// This rejection is used if the request body is syntactically valid JSON but couldn't be
    /// deserialized into the target type.
    pub struct JsonDataError(Error);
}

#[cfg(feature = "json")]
define_rejection! {
    #[status = BAD_REQUEST]
    #[body = "Failed to parse the request body as JSON"]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    /// Rejection type for [`Json`](crate::json::Json).
    ///
    /// This rejection is used if the request body didn't contain syntactically valid JSON.
    pub struct JsonSyntaxError(Error);
}

#[cfg(feature = "json")]
define_rejection! {
    #[status = UNSUPPORTED_MEDIA_TYPE]
    #[body = "Expected request with `Content-Type: application/json`"]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    /// Rejection type for [`Json`](crate::json::Json) used if the `Content-Type`
    /// header is missing.
    pub struct MissingJsonContentType;
}

#[cfg(feature = "json")]
composite_rejection! {
    /// Rejection used for [`Json`](crate::json::Json).
    ///
    /// Contains one variant for each way the [`Json`](crate::json::Json) extractor
    /// can fail.
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub enum JsonRejection {
        JsonDataError,
        JsonSyntaxError,
        MissingJsonContentType,
    }
}

/// Rejection used for [`TypedHeader`](super::TypedHeader).
#[derive(Debug)]
pub struct TypedHeaderRejection {
    pub(super) name: &'static http::header::HeaderName,
    pub(super) reason: TypedHeaderRejectionReason,
}

impl TypedHeaderRejection {
    /// Name of the header that caused the rejection
    pub fn name(&self) -> &http::header::HeaderName {
        self.name
    }

    /// Reason why the header extraction has failed
    pub fn reason(&self) -> &TypedHeaderRejectionReason {
        &self.reason
    }
}

/// Additional information regarding a [`TypedHeaderRejection`]
#[derive(Debug)]
#[non_exhaustive]
pub enum TypedHeaderRejectionReason {
    /// The header was missing from the HTTP request
    Missing,
    /// An error occured when parsing the header from the HTTP request
    Error(headers::Error),
}

impl IntoResponse for TypedHeaderRejection {
    fn into_response(self) -> Result<Response, RouteError> {
        (http::StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

impl std::fmt::Display for TypedHeaderRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reason {
            TypedHeaderRejectionReason::Missing => {
                write!(f, "Header of type `{}` was missing", self.name)
            }
            TypedHeaderRejectionReason::Error(err) => {
                write!(f, "{} ({})", err, self.name)
            }
        }
    }
}

impl std::error::Error for TypedHeaderRejection {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            TypedHeaderRejectionReason::Error(err) => Some(err),
            TypedHeaderRejectionReason::Missing => None,
        }
    }
}
