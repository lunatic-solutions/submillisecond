use http::{Method, Request, Response};
use std::mem;

use crate::util;

pub type HandlerFn<Req = String, Res = String> = fn(Request<Req>) -> Response<Res>;

#[derive(Clone)]
pub struct Router {
    handlers: Vec<(String, String, HandlerFn)>,
}

impl Router {
    pub fn new() -> Router {
        Router { handlers: vec![] }
    }

    pub fn as_raw(&self) -> Vec<(String, String, usize)> {
        self.handlers
            .iter()
            .map(|(method, path, handler)| {
                (method.clone(), path.clone(), *handler as *const () as usize)
            })
            .collect()
    }

    pub fn from_raw(raw: Vec<(String, String, usize)>) -> Router {
        let handlers = raw
            .iter()
            .map(|(method, path, handler)| {
                (method.clone(), path.clone(), {
                    unsafe {
                        let pointer = *handler as *const ();
                        mem::transmute::<*const (), HandlerFn>(pointer)
                    }
                })
            })
            .collect::<Vec<_>>();
        Self { handlers }
    }

    pub fn get(&mut self, path: &'static str, handler: HandlerFn) {
        self.handlers
            .push((Method::GET.to_string(), path.to_string(), handler));
    }

    pub fn post(&mut self, path: &'static str, handler: HandlerFn) {
        self.handlers
            .push((Method::POST.to_string(), path.to_string(), handler));
    }

    pub fn find_match(&self, request: &Request<String>) -> HandlerFn {
        for (method, path, handler) in self.handlers.iter() {
            if request.method().to_string() == *method && request.uri().to_string() == *path {
                return *handler;
            }
        }
        util::err_404
    }
}
