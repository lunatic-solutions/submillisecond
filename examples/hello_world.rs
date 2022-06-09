use std::collections::HashMap;

use http::HeaderMap;
use submillisecond::{
    extract::{path::Path, query::Query},
    get, Application,
};

#[get("/path/:id")]
fn path(Path(id): Path<String>) -> String {
    format!("Welcome, {id}")
}

#[get("/query")]
fn query(Query(query): Query<HashMap<String, String>>) -> String {
    query
        .into_iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[get("/header_map")]
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

fn main() {
    Application::build()
        .route(path)
        .route(query)
        .route(header_map)
        .listen(3000)
        .unwrap()
        .start_server();
}
