use std::convert::Infallible;

use crate::Request;

use super::FromRequest;

impl FromRequest for Request {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        Ok(Request {
            request: FromRequest::from_request(req).unwrap(),
            params: req.params.clone(),
            reader: req.reader.clone(),
        })
    }
}

impl FromRequest for http::Request<Vec<u8>> {
    type Rejection = Infallible;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        let mut req_builder = http::Request::builder();

        for (key, value) in req.headers() {
            req_builder = req_builder.header(key, value);
        }

        req_builder = req_builder
            .method(req.method().clone())
            .uri(req.uri().clone())
            .version(req.version());

        Ok(req_builder.body(req.body().clone()).unwrap())
    }
}
