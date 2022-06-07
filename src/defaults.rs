use crate::{Request, Response};

/// Return an error 404 not found response.
pub fn err_404(_: Request) -> Response {
    Response::builder()
        .status(404)
        .header("Content-Type", "HTML")
        .body(b"<h1>404: Not found</h1>".to_vec())
        .unwrap()
}
