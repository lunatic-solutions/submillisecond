use httparse::{self, Status};
use lunatic::net::TcpStream;
use std::{
    io::{BufReader, Read, Result as IoResult, Write},
    mem::MaybeUninit,
};

use crate::{Request, Response};

const MAX_HEADERS: usize = 96;
const REQUEST_BUFFER_SIZE: usize = 1024 * 8;

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

pub fn parse_request(stream: TcpStream) -> Request {
    let mut reader = BufReader::new(stream);
    let mut raw_request = Vec::with_capacity(REQUEST_BUFFER_SIZE);
    let buf = [0_u8; REQUEST_BUFFER_SIZE];

    let mut headers = unsafe {
        MaybeUninit::<[MaybeUninit<httparse::Header<'_>>; MAX_HEADERS]>::uninit().assume_init()
    };

    parse_request_chunks(&mut reader, &mut raw_request, &mut headers, buf).unwrap()
}

fn parse_request_chunks<'a>(
    reader: &'a mut BufReader<TcpStream>,
    raw_request: &'a mut Vec<u8>,
    headers: &'a mut [MaybeUninit<httparse::Header<'a>>; MAX_HEADERS],
    mut buf: [u8; REQUEST_BUFFER_SIZE],
) -> Result<Request, ParseRequestError> {
    let i = reader.read(&mut buf).unwrap();
    if i > 0 {
        raw_request.extend(&buf[..i]);
    }

    let mut req = httparse::Request::new(&mut []);

    let status = req
        .parse_with_uninit_headers(raw_request, headers)
        .map_err(ParseRequestError::HttpParseError)?;
    match status {
        Status::Complete(offset) => {
            let method =
                http::Method::try_from(req.method.ok_or(ParseRequestError::MissingMethod)?)
                    .map_err(|_| ParseRequestError::UnknownMethod)?;

            let mut request = Request::builder().method(method);

            if let Some(path) = req.path {
                request = request.uri(path);
            }

            request = req.headers.iter().fold(request, |request, header| {
                request.header(header.name, header.value)
            });

            request
                .body(raw_request[offset..].to_owned())
                .map_err(ParseRequestError::BadRequest)
        }
        Status::Partial => {
            todo!()
            // parse_request_chunks(reader, raw_request, headers, buf)
        }
    }
}

#[derive(Debug)]
pub enum ParseRequestError {
    BadRequest(http::Error),
    InvalidContentLengthHeader,
    MissingMethod,
    HttpParseError(httparse::Error),
    UnknownMethod,
}
