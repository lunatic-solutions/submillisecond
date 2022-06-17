use crate::{defaults, response::IntoResponse, Request, Response};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
pub struct Route(pub String);

#[derive(Debug)]
pub enum RouteError {
    ExtractorError(Response),
    RouteNotMatch(Request),
}

impl IntoResponse for RouteError {
    fn into_response(self) -> Response {
        match self {
            RouteError::ExtractorError(resp) => resp,
            RouteError::RouteNotMatch(req) => defaults::err_404(req),
        }
    }
}
