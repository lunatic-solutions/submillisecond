use http::Response;

pub trait IntoResponse<T> {
    fn into_response(self) -> Response<T>;
}
