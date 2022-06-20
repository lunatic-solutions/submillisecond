use crate::Request;

pub trait Guard {
    fn check(&self, req: &Request) -> bool;
}
