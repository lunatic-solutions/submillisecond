use std::collections::HashMap;

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

fn main() {
    Application::build()
        .route(path)
        .route(query)
        .listen(3000)
        .unwrap()
        .start_server();
}
