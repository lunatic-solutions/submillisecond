use crate::{Middleware, Request, Response};

pub static mut MIDDLEWARES: Vec<Box<dyn Middleware>> = vec![];

pub fn inject_middleware(middleware: Box<dyn Middleware>) {
    unsafe {
        MIDDLEWARES.push(middleware);
    }
}

pub fn run_before(req: &mut Request) {
    let middlewares = unsafe { &mut MIDDLEWARES };
    for mid in middlewares.iter_mut() {
        mid.before(req);
    }
}

pub fn drain(res: &mut Response) {
    let middlewares = unsafe { &MIDDLEWARES };
    for mid in middlewares.iter() {
        mid.after(res);
    }
}
