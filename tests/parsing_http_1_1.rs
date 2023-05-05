use std::io::{Read, Write};
use std::str::from_utf8;
use std::time::Duration;

use lunatic::net::TcpStream;
use lunatic::{sleep, test, Mailbox, Process};
use submillisecond::{router, Application, Body};

fn hell_world_server(port: u16, _: Mailbox<()>) {
    fn hello_world_handler() -> &'static str {
        "Hello world!"
    }

    Application::new(router! {
        GET "/" => hello_world_handler
    })
    .serve(format!("localhost:{port}"))
    .unwrap();
}

#[test]
fn empty_line_prefix_is_valid() {
    Process::spawn_link(8900, hell_world_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(10));
    let mut stream = TcpStream::connect("localhost:8900").unwrap();
    let request = "\r\n\nGET / HTTP/1.1\r\n\r\n".as_bytes();
    stream.write_all(request).unwrap();
    let mut response = [0u8; 256];
    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 200 OK\r\n\
        content-type: text/plain; charset=utf-8\r\n\
        content-length: 12\r\n\
        \r\n\
        Hello world!"
    );
}

#[test]
#[ignore]
fn pipeline_requests() {
    Process::spawn_link(8901, hell_world_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(10));
    let mut stream = TcpStream::connect("localhost:8901").unwrap();
    let request = "GET / HTTP/1.1\r\n\r\nGET / HTTP/1.1\r\n\r\n".as_bytes();
    stream.write_all(request).unwrap();
    let mut response = [0u8; 256];
    // First response
    let n = stream.read(&mut response[..100]).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 200 OK\r\n\
        content-type: text/plain; charset=utf-8\r\n\
        content-length: 12\r\n\
        \r\n\
        Hello world!"
    );
    // Second response
    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 200 OK\r\n\
        content-type: text/plain; charset=utf-8\r\n\
        content-length: 12\r\n\
        \r\n\
        Hello world!"
    );
}

#[test]
fn pipeline_requests_in_2_parts() {
    Process::spawn_link(8902, hell_world_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(10));
    let mut stream = TcpStream::connect("localhost:8902").unwrap();
    let request = "\
        GET / HTTP/1.1\r\n\
        Content-length: 5\r\n\
        \r\n\
        Hello\
        GET /"
        .as_bytes();
    stream.write_all(request).unwrap();
    let mut response = [0u8; 256];
    // First response
    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 200 OK\r\n\
        content-type: text/plain; charset=utf-8\r\n\
        content-length: 12\r\n\
        \r\n\
        Hello world!"
    );
    // Second response
    let request_rest = " HTTP/1.1\r\n\r\n".as_bytes();
    stream.write_all(request_rest).unwrap();

    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 200 OK\r\n\
        content-type: text/plain; charset=utf-8\r\n\
        content-length: 12\r\n\
        \r\n\
        Hello world!"
    );
}

#[test]
fn invalid_method() {
    Process::spawn_link(8903, hell_world_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(10));
    let mut stream = TcpStream::connect("localhost:8903").unwrap();
    let request = "INVALID / HTTP/1.1\r\n\r\n".as_bytes();
    stream.write_all(request).unwrap();
    let mut response = [0u8; 256];
    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 404 Not Found\r\n\
        content-type: text/html; charset=UTF-8\r\n\
        content-length: 23\r\n\
        \r\n\
        <h1>404: Not found</h1>"
    );
}

fn panic_server(port: u16, _: Mailbox<()>) {
    fn panic_handler() {
        panic!()
    }

    Application::new(router! {
        GET "/" => panic_handler
    })
    .serve(format!("localhost:{port}"))
    .unwrap();
}

#[test]
fn handler_panics() {
    Process::spawn_link(8904, panic_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(10));
    let mut stream = TcpStream::connect("localhost:8904").unwrap();
    let request = "GET / HTTP/1.1\r\n\r\n".as_bytes();
    stream.write_all(request).unwrap();
    let mut response = [0u8; 256];
    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 500 Internal Server Error\r\n\
        content-type: text/plain; charset=utf-8\r\n\
        content-length: 21\r\n\
        \r\n\
        Internal Server Error"
    );
}

fn post_echo_server(port: u16, _: Mailbox<()>) {
    fn hello_world_handler(data: Body) -> Vec<u8> {
        data.as_slice().into()
    }

    Application::new(router! {
        POST "/" => hello_world_handler
    })
    .serve(format!("localhost:{port}"))
    .unwrap();
}

#[test]
fn post_request_keep_alive() {
    Process::spawn_link(8905, post_echo_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(10));
    let mut stream = TcpStream::connect("localhost:8905").unwrap();
    let request = "\
        POST / HTTP/1.1\r\n\
        Content-length: 5\r\n\
        \r\n\
        Hello"
        .as_bytes();
    stream.write_all(request).unwrap();
    let mut response = [0u8; 256];
    let n = stream.read(&mut response).unwrap();
    let response_str = from_utf8(&response[..n]).unwrap();
    assert_eq!(
        response_str,
        "HTTP/1.1 200 OK\r\n\
        content-type: application/octet-stream\r\n\
        content-length: 5\r\n\
        \r\n\
        Hello"
    );
}
