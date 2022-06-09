use httparse;
use lunatic::net::TcpStream;
use std::io::{Read, Result as IoResult, Write};

use crate::{Request, Response};

pub fn write_response(mut stream: TcpStream, response: Response) -> IoResult<()> {
    // writing status line
    write!(
        &mut stream,
        "{:?} {} {}\r\n",
        response.version(),
        response.status().as_u16(),
        response.status().canonical_reason().unwrap()
    )?;
    // writing headers
    for (key, value) in response.headers().iter() {
        if let Ok(value) = String::from_utf8(value.as_ref().to_vec()) {
            write!(stream, "{}: {}\r\n", key, value)?;
        }
    }
    // separator between header and data
    write!(&mut stream, "\r\n")?;
    stream.write_all(response.body())?;
    Ok(())
}

pub fn parse_request(mut stream: TcpStream) -> Request {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut buf = [0; 200];
    if let Err(e) = stream.read(&mut buf) {
        panic!("[http reader] Failed to read from tcp stream {:?}", e);
    }
    let mut req = httparse::Request::new(&mut headers);
    let offset = req.parse(&buf).unwrap();
    if let (true, None) = (offset.is_partial(), req.path) {
        panic!("[http reader] Failed to read request");
    }

    let mut request_builder = Request::builder()
        .method(req.method.unwrap())
        .uri(req.path.unwrap());
    println!("[http reader] GOT THESE HEADERS {:?}", req);
    let mut content_length: usize = 1024;
    for h in req.headers {
        if h.name.is_empty() {
            break;
        }
        if h.name.to_lowercase() == "content-length" {
            content_length = h
                .value
                .iter()
                .map(|x| (x - b'0') as usize)
                .fold(0_usize, |acc, b| acc * 10 + b);
        }
        request_builder = request_builder.header(h.name, h.value);
    }
    // get body
    let mut body: Vec<u8> = Vec::with_capacity(content_length);
    if let httparse::Status::Complete(idx) = offset {
        body = buf[idx..usize::min(idx + content_length, buf.len())].to_owned();
    }

    // TODO: handle error if non-utf8 data received
    request_builder.body(body).unwrap()
}
