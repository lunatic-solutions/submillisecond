use http::Response;

pub trait IntoResponse {
    fn into_response(self) -> Response<Vec<u8>>;
}
