use http::{header, StatusCode};
use httparse::{self, Status};
use lunatic::net::TcpStream;
use std::{
    io::{BufReader, Read, Result as IoResult, Write},
    mem::MaybeUninit,
};

use crate::{response::IntoResponse, Response};

const MAX_HEADERS: usize = 96;
const REQUEST_BUFFER_SIZE: usize = 1024 * 8;
const REQUEST_MAX_SIZE: usize = 1024 * 8 * 512; // 512 kB

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

pub fn parse_request(stream: TcpStream) -> Result<http::Request<Vec<u8>>, ParseRequestError> {
    let mut reader = BufReader::new(stream);
    let mut raw_request = Vec::with_capacity(REQUEST_BUFFER_SIZE);
    let mut buf = [0_u8; REQUEST_BUFFER_SIZE];

    loop {
        let i = reader.read(&mut buf).unwrap();
        if i > 0 {
            raw_request.extend(&buf[..i]);
        }

        let mut headers = unsafe {
            MaybeUninit::<[MaybeUninit<httparse::Header<'_>>; MAX_HEADERS]>::uninit().assume_init()
        };
        let mut req = httparse::Request::new(&mut []);

        let status = req
            .parse_with_uninit_headers(&raw_request, &mut headers)
            .map_err(ParseRequestError::HttpParseError)?;
        match status {
            Status::Complete(offset) => {
                let method =
                    http::Method::try_from(req.method.ok_or(ParseRequestError::MissingMethod)?)
                        .map_err(|_| ParseRequestError::UnknownMethod)?;

                let mut request = http::Request::builder().method(method);

                if let Some(path) = req.path {
                    request = request.uri(path);
                }

                request = req.headers.iter().fold(request, |request, header| {
                    request.header(header.name, header.value)
                });

                let body = match request
                    .headers_ref()
                    .and_then(|headers| headers.get(&header::CONTENT_LENGTH))
                {
                    Some(content_length) => {
                        let length = content_length
                            .as_bytes()
                            .iter()
                            .map(|x| (*x as char).to_digit(10))
                            .fold(Some(0), |acc, b| Some(acc? * 10 + (b? as usize)))
                            .ok_or(ParseRequestError::InvalidContentLengthHeader)?;
                        raw_request[offset..offset + length].to_owned()
                    }
                    None => raw_request[offset..].to_owned(),
                };

                return request.body(body).map_err(ParseRequestError::BadRequest);
            }
            Status::Partial => {
                if raw_request.len() > REQUEST_MAX_SIZE {
                    return Err(ParseRequestError::RequestTooLarge);
                }
                continue;
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseRequestError {
    BadRequest(http::Error),
    HttpParseError(httparse::Error),
    InvalidContentLengthHeader,
    MissingMethod,
    RequestTooLarge,
    UnknownMethod,
}

impl IntoResponse for ParseRequestError {
    fn into_response(self) -> Response {
        match self {
            ParseRequestError::MissingMethod | ParseRequestError::UnknownMethod => {
                (StatusCode::METHOD_NOT_ALLOWED, ()).into_response()
            }
            _ => (StatusCode::BAD_REQUEST, ()).into_response(),
        }
    }
}

pub enum UriReadError {
    EmptyOnExpectedParam,
}

#[derive(Clone, Debug, Default)]
pub struct UriReader {
    uri: String,
    pub cursor: usize,
}

impl UriReader {
    pub fn new(uri: String) -> UriReader {
        UriReader { uri, cursor: 0 }
    }

    pub fn peek(&self, len: usize) -> &str {
        let read_attempt = self.cursor + len;
        if self.uri.len() >= read_attempt {
            return &self.uri[self.cursor..read_attempt];
        }
        ""
    }

    pub fn is_dangling_slash(&self) -> bool {
        self.uri.len() == self.cursor || &self.uri[self.cursor..] == "/"
    }

    pub fn read(&mut self, len: usize) {
        self.cursor += len;
    }

    pub fn read_matching(&mut self, s: &str) -> bool {
        let read_to = self.cursor + s.len();
        if read_to >= self.uri.len() {
            return false;
        }

        if &self.uri[self.cursor..read_to] == s {
            self.cursor = read_to;
            return true;
        }

        false
    }

    pub fn read_back(&mut self, len: usize) {
        self.cursor -= len;
    }

    pub fn ensure_next_slash(&mut self) -> bool {
        self.read_matching("/")
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn is_empty(&self, allow_trailing_slash: bool) -> bool {
        if allow_trailing_slash {
            self.uri.len() <= self.cursor || &self.uri[self.cursor..] == "/"
        } else {
            self.uri.len() <= self.cursor
        }
    }

    pub fn read_param(&mut self) -> Result<&str, UriReadError> {
        let initial_cursor = self.cursor;
        while !self.is_empty(false) {
            if self.peek(1) != "/" {
                self.read(1);
            } else {
                break;
            }
        }
        // if nothing was found, return error
        if initial_cursor == self.cursor {
            return Err(UriReadError::EmptyOnExpectedParam);
        }
        // read the param
        Ok(&self.uri[initial_cursor..self.cursor])
    }

    pub fn ends_with(&self, suffix: &str) -> bool {
        if self.cursor >= self.uri.len() {
            return false;
        }
        let end = &self.uri[self.cursor..];
        end == suffix
    }

    pub fn read_to_end(&self) -> &str {
        &self.uri[self.cursor..]
    }
}

#[cfg(test)]
mod tests {
    use super::UriReader;

    #[test]
    fn peek_empty_string() {
        let reader = UriReader::new("".to_string());
        assert_eq!(reader.peek(5), "");
    }

    #[test]
    fn peek_path() {
        let mut reader = UriReader::new("/alive".to_string());
        assert_eq!(reader.peek(3), "/al");
        reader.read(3);
        assert_eq!(reader.peek(3), "ive");
        reader.read(3);
        assert_eq!(reader.peek(3), "");
        reader.read(3);
    }
}
