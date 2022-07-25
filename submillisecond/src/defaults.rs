use crate::Response;

/// Return an error 404 not found response.
pub fn err_404() -> Response {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/html; charset=UTF-8")
        .body(b"<h1>404: Not found</h1>".to_vec())
        .unwrap()
}
