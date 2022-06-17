use http::uri::PathAndQuery;

use crate::{defaults, response::IntoResponse, Request, Response};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
pub struct Route(PathAndQuery);

impl Route {
    pub(crate) fn new(path_and_query: PathAndQuery) -> Self {
        Route(path_and_query)
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    pub fn matches(&self, route: &str) -> bool {
        self.0.path() == route
    }
}

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
