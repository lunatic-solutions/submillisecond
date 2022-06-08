use http::{header, HeaderValue, Request, StatusCode};
use serde::{de, ser, Serialize};

use crate::response::{IntoResponse, Response};

#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(bytes) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                bytes,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

pub fn from_json<T>(req: Request<T>) -> serde_json::Result<Request<Vec<u8>>>
where
    T: ser::Serialize,
{
    let (parts, body) = req.into_parts();
    let body = serde_json::to_vec(&body)?;
    Ok(Request::from_parts(parts, body))
}

pub fn to_json<T>(res: http::Response<Vec<u8>>) -> serde_json::Result<http::Response<T>>
where
    for<'de> T: de::Deserialize<'de>,
{
    let (parts, body) = res.into_parts();
    let body = serde_json::from_slice(&body)?;
    Ok(http::Response::from_parts(parts, body))
}
