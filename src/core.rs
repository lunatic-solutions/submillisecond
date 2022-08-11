use std::io::{BufReader, Read};
use std::mem::MaybeUninit;

use http::{header, StatusCode};
use httparse::{self, Status};
use lunatic::net::TcpStream;

use crate::response::IntoResponse;
use crate::Response;

const MAX_HEADERS: usize = 96;
const REQUEST_BUFFER_SIZE: usize = 1024 * 8;
const REQUEST_MAX_SIZE: usize = 1024 * 8 * 512; // 512 kB

pub(crate) fn parse_request(
    stream: TcpStream,
) -> Result<http::Request<Vec<u8>>, ParseRequestError> {
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
pub(crate) enum ParseRequestError {
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
