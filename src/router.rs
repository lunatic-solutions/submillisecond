use std::mem;

use http::uri::PathAndQuery;

use crate::{defaults, Request, Response};

pub type HandlerFn<Req = Vec<u8>, Res = Vec<u8>> =
    fn(Request<Req>) -> Result<Response<Res>, RouteError>;

#[derive(Clone)]
pub struct Router {
    handlers: Vec<HandlerFn>,
}

impl Router {
    pub fn new() -> Router {
        Router { handlers: vec![] }
    }

    pub fn as_raw(&self) -> Vec<usize> {
        self.handlers
            .iter()
            .map(|handler| *handler as *const () as usize)
            .collect()
    }

    pub fn from_raw(raw: Vec<usize>) -> Router {
        let handlers = raw
            .iter()
            .map(|handler| unsafe {
                let pointer = *handler as *const ();
                mem::transmute::<*const (), HandlerFn>(pointer)
            })
            .collect::<Vec<_>>();
        Self { handlers }
    }

    pub fn route(&mut self, handler: HandlerFn) {
        self.handlers.push(handler);
    }

    pub fn handle_request(&self, mut req: Request) -> Response {
        for handler in &self.handlers {
            match handler(req) {
                Ok(resp) => return resp,
                Err(RouteError::ExtractorError(resp)) => return resp,
                Err(RouteError::RouteNotMatch(request)) => req = request,
            }
        }
        defaults::err_404(req)
    }
}

impl Default for Router {
    fn default() -> Self {
        Router::new()
    }
}

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

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub struct __inernal__RouterNode {
    pub priority: u32,
    pub wild_child: bool,
    pub indices: Vec<u8>,
    pub node_type: __inernal__RouterNodeType,
    pub prefix: Vec<u8>,
    pub children: Vec<Self>,
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub enum __inernal__RouterNodeType {
    Root,
    Param,
    CatchAll,
    Static,
}
