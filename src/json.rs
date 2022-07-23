use http::{header, HeaderValue, StatusCode};
use serde::{ser, Serialize};

use crate::{
    response::{IntoResponse, Response},
    RequestContext,
};

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

pub fn from_json<T>(req: RequestContext) -> serde_json::Result<RequestContext>
where
    T: ser::Serialize,
{
    let params = req.params.clone();
    let reader = req.reader.clone();
    let (parts, body) = req.request.into_parts();
    let body = serde_json::to_vec(&body)?;
    Ok(RequestContext {
        request: http::Request::from_parts(parts, body),
        params,
        reader,
        next: req.next,
    })
}

pub fn to_json(res: Response) -> serde_json::Result<Response> {
    let (parts, body) = res.into_parts();
    let body = serde_json::from_slice(&body)?;
    Ok(http::Response::from_parts(parts, body))
}

pub(crate) fn json_content_type(req: &RequestContext) -> bool {
    let content_type = if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
        content_type
    } else {
        return false;
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return false;
    };

    let is_json_content_type = mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().map_or(false, |name| name == "json"));

    is_json_content_type
}
