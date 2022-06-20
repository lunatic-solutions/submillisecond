use http::{header, StatusCode};
use httparse::{self, Status};
use lunatic::{net::{TcpStream, TcpListener, ToSocketAddrs}, Mailbox, Process};
use submillisecond_core::router::params::Params;
use std::{
    io::{BufReader, Read, Result as IoResult, Write, self},
    mem::MaybeUninit,
};

use crate::{response::IntoResponse, Request, Response, router::RouteError};

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

pub fn parse_request(stream: TcpStream) -> Result<Request, ParseRequestError> {
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

                let mut request = Request::builder().method(method);

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

pub struct UriReader {
    uri: String,
    cursor: usize,
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
        return &""
    }

    pub fn read(&mut self, len: usize) -> &str {
        let read_attempt = self.cursor + len;
        if self.uri.len() >= read_attempt {
            let s = &self.uri[self.cursor..read_attempt];
            self.cursor += len;
            return s;
        }
        return &""
    }

    pub fn is_empty(&self) -> bool {
        self.uri.len() <= self.cursor
    }

    pub fn read_param(&mut self) -> Result<&str, String> {
        let initial_cursor = self.cursor;
        while !self.is_empty() {
            // println!("PEEKING PARAM {}", self.peek(1));
            if self.peek(1) != "/" {
                self.read(1);
            } else {
                break;
            }
        }
        // if nothing was found, return error
        if initial_cursor == self.cursor {
            return Err("Failed to read param".to_string());
        }
        // read the param
        println!("JUST READ PARAM {} | after: {}", &self.uri[initial_cursor..(self.cursor - initial_cursor)], self.peek(1));
        Ok(&self.uri[initial_cursor..self.cursor])
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
        assert_eq!(reader.read(3), "/al");
        assert_eq!(reader.peek(3), "ive");
        assert_eq!(reader.read(3), "ive");
        assert_eq!(reader.peek(3), "");
        assert_eq!(reader.read(3), "");
    }
}

pub trait WebApp {
    fn handle_get_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }
    fn handle_post_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }
    fn handle_put_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }
    fn handle_patch_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }
    fn handle_delete_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }

    fn handle_options_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }

    fn handle_head_request(request: Request, _: &mut Params) -> Result<Response, RouteError> {
        Err(RouteError::RouteNotMatch(
            request,
        ))
    }

    fn handle_request(stream: TcpStream, _: Mailbox<()>) -> () {
    let request = match parse_request(stream.clone()) {
        Ok(request) => request,
        Err(err) => {
            if let Err(err) = write_response(stream, err.into_response()) {
                eprintln!("[http reader] Failed to send response {:?}", err);
            }
            return;
        }
    };
    let mut params = ::submillisecond_core::router::params::Params::new();
    let http_version = request.version();
    
    // invoke generated handlers
    let mut response: Response = {
        match *request.method() {
            http::Method::GET => Self::handle_get_request(request, &mut params),
            http::Method::POST => Self::handle_post_request(request, &mut params),
            http::Method::PUT => Self::handle_put_request(request, &mut params),
            http::Method::PATCH => Self::handle_patch_request(request, &mut params),
            http::Method::DELETE => Self::handle_delete_request(request, &mut params),
            http::Method::OPTIONS => Self::handle_options_request(request, &mut params),
            http::Method::HEAD => Self::handle_head_request(request, &mut params),
            
            _ => Err(RouteError::RouteNotMatch(
                request,
            )),
        }
    }.unwrap_or_else(|err| err.into_response());

    let content_length = response.body().len();
    *response.version_mut() = http_version;
    response.headers_mut().append(
        ::http::header::CONTENT_LENGTH,
        ::http::HeaderValue::from(content_length),
    );
    if let Err(err) = write_response(stream, response) {
        eprintln!("[http reader] Failed to send response {:?}", err);
    }
    }

    fn serve<A: ToSocketAddrs>(addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;

        while let Ok((stream, _)) = listener.accept() {
            Process::spawn_link(
                stream,
                Self::handle_request
            );
        }

        Ok(())
    }

    fn merge_extensions(request: &mut Request, params: &mut Params) -> () {
        let extensions = request.extensions_mut();
        match extensions.get_mut::<::submillisecond_core::router::params::Params>() {
            Some(ext_params) => {
                ext_params.merge(params.clone());
            },
            None => {
                extensions.insert(params.clone());
            }
        };
    }
}