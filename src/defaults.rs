use http::{Request, Response};

/// Return an error 404 not found response.
pub fn err_404(_: Request<String>) -> Response<String> {
    Response::builder()
        .status(404)
        .header("Content-Type", "HTML")
        .body("<h1>404: Not found</h1>".to_string())
        .unwrap()
}
