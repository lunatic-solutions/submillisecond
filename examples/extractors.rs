use std::collections::HashMap;

use http::HeaderMap;
use serde::Deserialize;
use submillisecond::extract::{Host, Path, Query, Splat};
use submillisecond::params::Params;
use submillisecond::{router, Application, Json, NamedParam, TypedHeader};

fn index() -> &'static str {
    "Hello :)"
}

fn path(Path(id): Path<String>) -> String {
    format!("Welcome, {id}")
}

fn splat(Splat(splat): Splat) -> String {
    splat
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

fn host(Host(host): Host) -> String {
    host
}

fn typed_header(TypedHeader(host): TypedHeader<headers::Host>) -> String {
    host.to_string()
}

fn params(params: Params) -> String {
    let name = params.get("name").unwrap_or("user");
    let age = params.get("age").unwrap_or("age");
    format!("Welcome, {name}. You are {age} years old.")
}

#[derive(NamedParam)]
#[param(name = "age")]
struct AgeParam(i32);

fn named_param(AgeParam(age): AgeParam) -> String {
    format!("You are {age} years old")
}

#[derive(NamedParam)]
struct NamedParamStruct {
    name: String,
    age: i32,
}

fn named_param2(NamedParamStruct { name, age }: NamedParamStruct) -> String {
    format!("Hi {name}, you are {age} years old")
}

fn string(body: String) -> String {
    body
}

fn vec(body: Vec<u8>) -> Vec<u8> {
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

fn main() -> std::io::Result<()> {
    Application::new(router! {
        GET "/" => index
        GET "/queries" => query
        GET "/header_map" => header_map
        GET "/host" => host
        GET "/typed_header" => typed_header
        GET "/params/:name/:age" => params
        GET "/named_param/:age" => named_param
        GET "/named_param2/:name/:age" => named_param2
        GET "/path/:id" => path
        GET "/splat-*" => splat
        POST "/string" => string
        POST "/vec" => vec
        POST "/json" => json
    })
    .serve("0.0.0.0:3000")
}
