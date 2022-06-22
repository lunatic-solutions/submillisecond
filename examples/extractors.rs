use std::{collections::HashMap, io};

use headers::Host;
use http::HeaderMap;
use serde::Deserialize;
use submillisecond::{
    extract::{Path, Query, TypedHeader},
    json::Json,
    router, Application, Request,
};

fn index() -> &'static str {
    "Hello :)"
}

fn path(Path(id): Path<String>) -> String {
    format!("Welcome, {id}")
}

fn query(Query(query): Query<HashMap<String, String>>) -> String {
    query
        .into_iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn header_map(headers: HeaderMap) -> String {
    headers
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}: {}",
                key.map(|key| key.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                value.to_str().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn typed_header(TypedHeader(host): TypedHeader<Host>) -> String {
    host.to_string()
}

fn string(req: Request, body: String) -> String {
    assert!(req.body().is_empty()); // Taking body with `String` extractor should leave the request body empty
    body
}

fn vec(req: Request, body: Vec<u8>) -> Vec<u8> {
    println!("{}", body.len());
    assert!(req.body().is_empty()); // Taking body with `Vec<u8>` extractor should leave the request body empty
    body
}

#[derive(Deserialize, Debug)]
struct Login {
    email: String,
    password: String,
}

fn json(Json(login): Json<Login>) -> String {
    format!("Email: {}\nPassword: {}", login.email, login.password)
}

fn main() -> io::Result<()> {
    Application::new(router! {
        GET "/" => index
        GET "/querys" => query
        GET "/header_map" => header_map
        GET "/typed_header" => typed_header
        POST "/string" => string
        POST "/vec" => vec
        POST "/json" => json
    })
    .serve("0.0.0.0:3000")
}
