use crate::RequestContext;

pub trait Guard {
    fn check(&self, req: &RequestContext) -> bool;
}
