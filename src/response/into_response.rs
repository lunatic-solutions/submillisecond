use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt;

use http::header::{self, HeaderName};
use http::{Extensions, HeaderMap, HeaderValue, StatusCode};

use super::{IntoResponseParts, Response, ResponseParts};

/// Converts a type into a [`Response`].
///
/// Types implementing `IntoResponse` can be returned from handlers.
pub trait IntoResponse: Sized {
    /// Create a response.
    fn into_response(self) -> Response;
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.status_mut() = self;
        res
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::default()
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response {
        match self {}
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
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

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        Cow::Borrowed(self).into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Cow::<'static, str>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, str> {
    fn into_response(self) -> Response {
        Response::builder()
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
            )
            .body(self.as_bytes().to_vec())
            .unwrap()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Response {
        Cow::Borrowed(self).into_response()
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        Cow::<'static, [u8]>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, [u8]> {
    fn into_response(self) -> Response {
        Response::builder()
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
            )
            .body(self.to_vec())
            .unwrap()
    }
}

/// Html response body with Content-Type set to utf8 text/html.
pub struct Html<T>(pub T);

impl<T> IntoResponse for Html<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        let mut res = self.0.into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
        );
        res
    }
}

impl<R> IntoResponse for (StatusCode, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let mut res = self.1.into_response();
        *res.status_mut() = self.0;
        res
    }
}

impl IntoResponse for HeaderMap {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.headers_mut() = self;
        res
    }
}

impl IntoResponse for Extensions {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.extensions_mut() = self;
        res
    }
}

impl<K, V, const N: usize> IntoResponse for [(K, V); N]
where
    K: TryInto<HeaderName>,
    K::Error: fmt::Display,
    V: TryInto<HeaderValue>,
    V::Error: fmt::Display,
{
    fn into_response(self) -> Response {
        (self, ()).into_response()
    }
}

impl<R> IntoResponse for (http::response::Parts, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let (parts, res) = self;
        (parts.status, parts.headers, parts.extensions, res).into_response()
    }
}

impl<R> IntoResponse for (http::response::Response<()>, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let (template, res) = self;
        let (parts, ()) = template.into_parts();
        (parts, res).into_response()
    }
}

/// Response for redirecting the client
///
/// # Example
/// ```
/// use submillisecond::http::Uri;
/// Redirect::to(Uri::from_static("https://lunatic.solutions/"));
/// ```
pub struct Redirect {
    /// Uri to redirect to
    pub uri: http::Uri,
    status_code: StatusCode,
}

impl Redirect {
    /// Constructs a **303 See Other** redirect response.
    ///
    /// Instructs the client to issue a GET request to the supplied URI.
    ///
    /// [[RFC7231, Section 6.4.4](https://tools.ietf.org/html/rfc7231#section-6.4.4)]
    pub fn to(uri: http::Uri) -> Self {
        Redirect {
            uri,
            status_code: StatusCode::SEE_OTHER,
        }
    }

    /// Constructs a **307 Temporary Redirect** redirect response.
    ///
    /// Instructs the client to reissue the original request to the supplied URI, while keeping the original HTTP method and request content.
    ///
    /// [[RFC7231, Section 6.4.7](https://tools.ietf.org/html/rfc7231#section-6.4.7)]
    pub fn temporary(uri: http::Uri) -> Self {
        Redirect {
            uri,
            status_code: StatusCode::TEMPORARY_REDIRECT,
        }
    }

    /// Constructs a **308 Permanent Redirect** redirect response.
    ///
    /// Instructs the client to reissue the original request to the supplied URI, while keeping the original HTTP method and request content.
    ///
    /// Should only be used for permanent redirects, since it may be cached by the client.
    ///
    /// [[RFC7238](https://tools.ietf.org/html/rfc7238)]
    pub fn permanent(uri: http::Uri) -> Self {
        Redirect {
            uri,
            status_code: StatusCode::PERMANENT_REDIRECT,
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        Response::builder()
            .status(self.status_code)
            .header("Location", self.uri.to_string())
            .body(Vec::new())
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use lunatic::test;

    use super::{Html, IntoResponse, Redirect};

    #[test]
    fn redirect() {
        let uri = "https://lunatic.solutions/";

        let base_redirect = Redirect::to(http::Uri::from_static(uri)).into_response();
        let temporary_redirect = Redirect::temporary(http::Uri::from_static(uri)).into_response();
        let permanent_redirect = Redirect::permanent(http::Uri::from_static(uri)).into_response();

        assert_eq!(base_redirect.status(), http::StatusCode::SEE_OTHER);
        assert_eq!(
            temporary_redirect.status(),
            http::StatusCode::TEMPORARY_REDIRECT
        );
        assert_eq!(
            permanent_redirect.status(),
            http::StatusCode::PERMANENT_REDIRECT
        );

        assert!(base_redirect.headers().contains_key("Location"));
        assert!(temporary_redirect.headers().contains_key("Location"));
        assert!(permanent_redirect.headers().contains_key("Location"));

        assert_eq!(base_redirect.headers().get("Location").unwrap(), uri);
        assert_eq!(temporary_redirect.headers().get("Location").unwrap(), uri);
        assert_eq!(permanent_redirect.headers().get("Location").unwrap(), uri);
    }

    #[test]
    fn html_response() {
        let response = Html("<!DOCTYPE html>".to_string()).into_response();
        let headers: Vec<_> = response.headers().get_all("content-type").iter().collect();
        assert_eq!(headers, vec!["text/html; charset=utf-8"]);
        assert_eq!(response.body(), b"<!DOCTYPE html>");
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
            fn into_response(self) -> Response {
                let ($($ty),*, res) = self;

                let res = res.into_response();
                let parts = ResponseParts { res };

                $(
                    let parts = match $ty.into_response_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_response();
                        }
                    };
                )*

                parts.res
            }
        }

        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoResponse for (StatusCode, $($ty),*, R)
        where
            $( $ty: IntoResponseParts, )*
            R: IntoResponse,
        {
            fn into_response(self) -> Response {
                let (status, $($ty),*, res) = self;

                let res = res.into_response();
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
            fn into_response(self) -> Response {
                let (outer_parts, $($ty),*, res) = self;

                let res = res.into_response();
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
            fn into_response(self) -> Response {
                let (template, $($ty),*, res) = self;
                let (parts, ()) = template.into_parts();
                (parts, $($ty),*, res).into_response()
            }
        }
    }
}

all_the_tuples!(impl_into_response);
