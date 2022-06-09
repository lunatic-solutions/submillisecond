use std::{convert::Infallible, ops::Deref};

use headers::HeaderMapExt;

use crate::{
    extract::FromRequest,
    response::{IntoResponse, IntoResponseParts, ResponseParts},
    Request, Response,
};

#[derive(Debug, Clone, Copy)]
pub struct TypedHeader<T>(pub T);

impl<T> FromRequest for TypedHeader<T>
where
    T: headers::Header,
{
    type Rejection = TypedHeaderRejection;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        match req.headers().typed_try_get::<T>() {
            Ok(Some(value)) => Ok(Self(value)),
            Ok(None) => Err(TypedHeaderRejection {
                name: T::name(),
                reason: TypedHeaderRejectionReason::Missing,
            }),
            Err(err) => Err(TypedHeaderRejection {
                name: T::name(),
                reason: TypedHeaderRejectionReason::Error(err),
            }),
        }
    }
}

impl<T> Deref for TypedHeader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> IntoResponseParts for TypedHeader<T>
where
    T: headers::Header,
{
    type Error = Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.headers_mut().typed_insert(self.0);
        Ok(res)
    }
}

impl<T> IntoResponse for TypedHeader<T>
where
    T: headers::Header,
{
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        res.headers_mut().typed_insert(self.0);
        res
    }
}

/// Rejection used for [`TypedHeader`](super::TypedHeader).
#[derive(Debug)]
pub struct TypedHeaderRejection {
    name: &'static http::header::HeaderName,
    reason: TypedHeaderRejectionReason,
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
    fn into_response(self) -> Response {
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
