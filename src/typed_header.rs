use std::convert::Infallible;
use std::ops::Deref;

use headers::HeaderMapExt;

use crate::extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason};
use crate::extract::FromRequest;
use crate::response::{IntoResponse, IntoResponseParts, ResponseParts};
use crate::{RequestContext, Response};

/// Extractor and response that works with typed header values from [`headers`].
///
/// # As extractor
///
/// In general, it's recommended to extract only the needed headers via
/// `TypedHeader` rather than removing all headers with the `HeaderMap`
/// extractor.
///
/// ```rust,no_run
/// use submillisecond::{router, TypedHeader, headers::UserAgent};
///
/// fn users_teams_show(
///     TypedHeader(user_agent): TypedHeader<UserAgent>,
/// ) {
///     // ...
/// }
///
/// router! {
///     GET "/users/:user_id/team/:team_id" => users_teams_show
/// }
/// ```
///
/// # As response
///
/// ```rust
/// use submillisecond::{TypedHeader, headers::ContentType};
///
/// fn handler() -> (TypedHeader<ContentType>, &'static str) {
///     (
///         TypedHeader(ContentType::text_utf8()),
///         "Hello, World!",
///     )
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TypedHeader<T>(pub T);

impl<T> FromRequest for TypedHeader<T>
where
    T: headers::Header,
{
    type Rejection = TypedHeaderRejection;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
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
