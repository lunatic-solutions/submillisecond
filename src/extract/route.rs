use std::convert::Infallible;

use super::FromRequest;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
pub struct Route(pub String);

impl FromRequest for Route {
    type Rejection = Infallible;

    fn from_request(req: &mut crate::Request) -> Result<Self, Self::Rejection> {
        Ok(Route(req.uri().path().to_string()))
    }
}
