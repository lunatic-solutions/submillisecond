use http::{Request, Response};

/// Return an error 404 not found response.
pub fn err_404(_: Request<String>) -> Response<Vec<u8>> {
    Response::builder()
        .status(404)
        .header("Content-Type", "HTML")
        .body(b"<h1>404: Not found</h1>".to_vec())
        .unwrap()
}
