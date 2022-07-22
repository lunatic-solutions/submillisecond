use std::{borrow::Cow, convert::Infallible, fmt};

use http::{
    header::{self, HeaderName},
    Extensions, HeaderMap, HeaderValue, StatusCode,
};

use crate::{defaults, RouteError};

use super::{IntoResponseParts, Response, ResponseParts};

/// Converts a type into a [`Response`].
///
/// Types implementing `IntoResponse` can be returned from handlers.
pub trait IntoResponse: Sized {
    /// Create a response.
    fn into_response(self) -> Result<Response, RouteError>;

    /// Creates a final response by converting any errors into a response.
    fn into_final_response(self) -> Response {
        match self.into_response() {
            Ok(res) => res,
            Err(RouteError::ExtractorError(resp)) => resp,
            Err(RouteError::RouteNotMatch(_)) => defaults::err_404(),
        }
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Result<Response, RouteError> {
        ().into_response().map(|mut res| {
            *res.status_mut() = self;
            res
        })
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response, RouteError> {
        Ok(Response::default())
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Result<Response, RouteError> {
        match self {}
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response, RouteError> {
        Ok(self)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Result<Response, RouteError> {
        match self {
            Ok(value) => value.into_response(),
            Err(err) => err.into_response(),
        }
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response, RouteError> {
        Cow::Borrowed(self).into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response, RouteError> {
        Cow::<'static, str>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, str> {
    fn into_response(self) -> Result<Response, RouteError> {
        Ok(Response::builder()
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
            )
            .body(self.as_bytes().to_vec())
            .unwrap())
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response, RouteError> {
        Cow::Borrowed(self).into_response()
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response, RouteError> {
        Cow::<'static, [u8]>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, [u8]> {
    fn into_response(self) -> Result<Response, RouteError> {
        Ok(Response::builder()
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
            )
            .body(self.to_vec())
            .unwrap())
    }
}

impl<R> IntoResponse for (StatusCode, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Result<Response, RouteError> {
        self.1.into_response().map(|mut res| {
            *res.status_mut() = self.0;
            res
        })
    }
}

impl IntoResponse for HeaderMap {
    fn into_response(self) -> Result<Response, RouteError> {
        ().into_response().map(|mut res| {
            *res.headers_mut() = self;
            res
        })
    }
}

impl IntoResponse for Extensions {
    fn into_response(self) -> Result<Response, RouteError> {
        ().into_response().map(|mut res| {
            *res.extensions_mut() = self;
            res
        })
    }
}

impl<K, V, const N: usize> IntoResponse for [(K, V); N]
where
    K: TryInto<HeaderName>,
    K::Error: fmt::Display,
    V: TryInto<HeaderValue>,
    V::Error: fmt::Display,
{
    fn into_response(self) -> Result<Response, RouteError> {
        (self, ()).into_response()
    }
}

impl<R> IntoResponse for (http::response::Parts, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Result<Response, RouteError> {
        let (parts, res) = self;
        (parts.status, parts.headers, parts.extensions, res).into_response()
    }
}

impl<R> IntoResponse for (http::response::Response<()>, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Result<Response, RouteError> {
        let (template, res) = self;
        let (parts, ()) = template.into_parts();
        (parts, res).into_response()
    }
}

macro_rules! impl_into_response {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoResponse for ($($ty),*, R)
        where
            $( $ty: IntoResponseParts, )*
            R: IntoResponse,
        {
            fn into_response(self) -> Result<Response, RouteError> {
                let ($($ty),*, res) = self;

                let res = res.into_response()?;
                let parts = ResponseParts { res };

                $(
                    let parts = match $ty.into_response_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_response();
                        }
                    };
                )*

                Ok(parts.res)
            }
        }

        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoResponse for (StatusCode, $($ty),*, R)
        where
            $( $ty: IntoResponseParts, )*
            R: IntoResponse,
        {
            fn into_response(self) -> Result<Response, RouteError> {
                let (status, $($ty),*, res) = self;

                let res = res.into_response()?;
                let parts = ResponseParts { res };

                $(
                    let parts = match $ty.into_response_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_response();
                        }
                    };
                )*

                (status, parts.res).into_response()
            }
        }

        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoResponse for (http::response::Parts, $($ty),*, R)
        where
            $( $ty: IntoResponseParts, )*
            R: IntoResponse,
        {
            fn into_response(self) -> Result<Response, RouteError> {
                let (outer_parts, $($ty),*, res) = self;

                let res = res.into_response()?;
                let parts = ResponseParts { res };
                $(
                    let parts = match $ty.into_response_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_response();
                        }
                    };
                )*

                (outer_parts, parts.res).into_response()
            }
        }

        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoResponse for (http::response::Response<()>, $($ty),*, R)
        where
            $( $ty: IntoResponseParts, )*
            R: IntoResponse,
        {
            fn into_response(self) -> Result<Response, RouteError> {
                let (template, $($ty),*, res) = self;
                let (parts, ()) = template.into_parts();
                (parts, $($ty),*, res).into_response()
            }
        }
    }
}

all_the_tuples!(impl_into_response);
