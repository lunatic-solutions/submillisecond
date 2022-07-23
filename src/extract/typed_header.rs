use std::{convert::Infallible, ops::Deref};

use headers::HeaderMapExt;

use crate::{
    extract::FromRequest,
    response::{IntoResponse, IntoResponseParts, ResponseParts},
    RequestContext, Response,
};

use super::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason};

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
