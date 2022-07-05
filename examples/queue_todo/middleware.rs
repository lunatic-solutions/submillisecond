use submillisecond::Middleware;

// =====================================
// Middleware for requests
// =====================================
#[derive(Default)]
pub struct LoggingMiddleware {
    request_id: String,
}

impl Middleware for LoggingMiddleware {
    fn before(&mut self, req: &mut submillisecond::Request) {
        self.request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|req_id| req_id.to_str().ok())
            .map(|req_id| req_id.to_string())
            .unwrap_or_else(|| "DEFAULT_REQUEST_ID".to_string());
        println!("[ENTER] request {}", self.request_id);
    }

    fn after(&self, _res: &mut submillisecond::Response) {
        println!("[EXIT] request {}", self.request_id);
    }
}
