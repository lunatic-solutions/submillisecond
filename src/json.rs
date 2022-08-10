use http::{header, HeaderValue, StatusCode};
use serde::Serialize;

use crate::response::{IntoResponse, Response};
use crate::RequestContext;

/// Json can be used as an extractor, or response type.
///
/// When used as an extractor, the request body will be deserialized into inner
/// type `T` with [`serde::Deserialize`].
///
/// For returning `Json`, the inner type `T` will be serialized into the
/// response body with [`serde::Serialize`], and the `Content-Type` header will
/// be set to `application/json`.
///
/// # Extractor example
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct LoginPayload {
///     email: String,
///     password: String,
/// }
///
/// fn login(Json(login): Json<LoginPayload>) -> String {
///     format!("Email: {}\nPassword: {}", login.email, login.password)
/// }
/// ```
///
/// # Response example
///
/// ```
/// use serde::Serialize;
/// use submillisecond::extract::Path;
///
/// #[derive(Serialize)]
/// struct User {
///     email: String,
///     password: String,
/// }
///
/// fn get_user(Path(id): Path<u32>) -> Json<User> {
///     let user = find_user(id);
///     Json(user)
/// }
/// ```
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
