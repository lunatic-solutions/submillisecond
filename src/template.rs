use crate::response::{IntoResponse, Response};

#[derive(Debug, Clone, Copy, Default)]
pub struct Template<T>(pub T);

impl<T> IntoResponse for Template<T>
where
    T: askama::Template,
{
    fn into_response(self) -> Response {
        self.0.render().into_response()
    }
}

impl IntoResponse for &dyn askama::DynTemplate {
    fn into_response(self) -> Response {
        self.dyn_render().into_response()
    }
}

impl IntoResponse for askama::Error {
    fn into_response(self) -> Response {
        let mut res = ().into_response();
        *res.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
        res
    }
}
