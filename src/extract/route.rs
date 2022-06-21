use std::convert::Infallible;

use crate::router::Route;

use super::FromRequest;

impl FromRequest for Route {
    type Rejection = Infallible;

    fn from_request(req: &mut crate::Request) -> Result<Self, Self::Rejection> {
        Ok(req.extensions().get::<Route>().unwrap().clone())
    }
}
