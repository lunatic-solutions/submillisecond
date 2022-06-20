use std::collections::HashMap;

use headers::Host;
use http::HeaderMap;
use serde::Deserialize;
use submillisecond::{
    extract::{Path, Query, TypedHeader},
    get,
    json::Json,
    post, Application, NamedParam, Request,
};

// #[derive(NamedParam)]
// #[param(name = "age")]
struct AgeParam(i32);

impl ::submillisecond::extract::FromRequest for AgeParam {
    type Rejection = ::submillisecond::extract::rejection::MissingPathParams;

    fn from_request(
        req: &mut ::submillisecond::Request,
    ) -> ::std::result::Result<Self, Self::Rejection> {
        let param = req
            .extensions_mut()
            .get::<::submillisecond_core::router::params::Params>()
            .unwrap()
            .get("age")
            .ok_or(::submillisecond::extract::rejection::MissingPathParams)?;

        todo!()
    }
}

fn main() {
    Application::build().listen(3000).unwrap().start_server();
}
