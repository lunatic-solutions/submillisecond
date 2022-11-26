use std::io::Read;

use httparse::{self, Status, EMPTY_HEADER};
use lunatic::net::TcpStream;

const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024;
const REQUEST_BUFFER_SIZE: usize = 4096;
const MAX_HEADERS: usize = 128;

/// The request body.
#[derive(Debug, Clone, Copy)]
pub struct Body<'a>(&'a [u8]);

impl<'a> Body<'a> {
    /// Create a request body from a slice.
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self(slice)
    }

    /// Returns the request body as a slice.
    pub fn as_slice(&self) -> &[u8] {
        self.0
    }

    /// Returns the body length in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if body is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The result of parsing a request from a buffer.
type RequestResult<'a> = Result<http::Request<Body<'a>>, ParseRequestError>;
/// Data belonging to the next request.
type NextRequest = Vec<u8>;

/// One or more HTTP request.
///
/// One TCP read can yield multiple pipelined requests. We keep the data of the
/// next request(s) around (without parsing it) and seed the next handler
/// process with it.
pub(crate) struct PipelinedRequests<'a> {
    request: RequestResult<'a>,
    next: NextRequest,
}

impl<'a> PipelinedRequests<'a> {
    /// Returns the result of parsing the first request + data belonging to
    /// other pipelined requests.
    pub(crate) fn pipeline(self) -> (RequestResult<'a>, NextRequest) {
        (self.request, self.next)
    }
}

impl<'a> PipelinedRequests<'a> {
    /// A complete request means **only one** complete request is the buffer and
    /// no pipelined requests.
    fn from_complete(request: http::Request<Body<'a>>) -> Self {
        PipelinedRequests {
            request: Ok(request),
            next: Vec::new(),
        }
    }

    /// One complete request and data belonging to others is contained in the
    /// buffer.
    fn from_pipeline(request: http::Request<Body<'a>>, next: Vec<u8>) -> Self {
        PipelinedRequests {
            request: Ok(request),
            next,
        }
    }

    /// If the first request can't be parsed correctly, it doesn't make sense to
    /// attempt parsing pipelined requests.
    fn from_err(err: ParseRequestError) -> Self {
        PipelinedRequests {
            request: Err(err),
            next: Vec::new(),
        }
    }
}

pub(crate) fn parse_requests<'a>(
    request_buffer: &'a mut Vec<u8>,
    stream: &mut TcpStream,
) -> PipelinedRequests<'a> {
    let mut buffer = [0_u8; REQUEST_BUFFER_SIZE];
    let mut headers = [EMPTY_HEADER; MAX_HEADERS];

    // Loop until at least one complete request is read.
    let (request_raw, offset) = loop {
        // In case of pipelined requests the `request_buffer` is going to come
        // prefilled with some data, and we should attempt to parse it into a request
        // before we decide to read more from `TcpStream`.
        let mut request_raw = httparse::Request::new(&mut headers);
        match request_raw.parse(request_buffer) {
            Ok(state) => match state {
                Status::Complete(offset) => {
                    // Continue outside the loop.
                    break (request_raw, offset);
                }
                Status::Partial => {
                    // Read more data from TCP stream
                    let n = stream.read(&mut buffer);
                    if n.is_err() || *n.as_ref().unwrap() == 0 {
                        if request_buffer.is_empty() {
                            return PipelinedRequests::from_err(
                                ParseRequestError::TcpStreamClosedWithoutData,
                            );
                        } else {
                            return PipelinedRequests::from_err(ParseRequestError::TcpStreamClosed);
                        }
                    }
                    // Invalidate references in `headers` that could point to the previous
                    // `request_buffer` before extending it.
                    headers = [EMPTY_HEADER; MAX_HEADERS];
                    request_buffer.extend(&buffer[..(n.unwrap())]);
                    // If request passed max size, abort
                    if request_buffer.len() > MAX_REQUEST_SIZE {
                        return PipelinedRequests::from_err(ParseRequestError::RequestTooLarge);
                    }
                }
            },
            Err(err) => {
                return PipelinedRequests::from_err(ParseRequestError::HttpParseError(err));
            }
        }
    };

    // At this point one full request header is available, but the body (if it
    // exists) might not be fully loaded yet.

    let method = match http::Method::try_from(request_raw.method.unwrap()) {
        Ok(method) => method,
        Err(_) => {
            return PipelinedRequests::from_err(ParseRequestError::UnknownMethod);
        }
    };
    let request = http::Request::builder()
        .method(method)
        .uri(request_raw.path.unwrap());
    let mut content_length = None;
    let request = request_raw.headers.iter().fold(request, |request, header| {
        if header.name.to_lowercase() == "content-length" {
            let value_string = std::str::from_utf8(header.value).unwrap();
            let length = value_string.parse::<usize>().unwrap();
            content_length = Some(length);
        }
        request.header(header.name, header.value)
    });
    // If content-length exists, request has a body
    if let Some(content_length) = content_length {
        #[allow(clippy::comparison_chain)]
        if request_buffer[offset..].len() == content_length {
            // Complete content is captured from the request w/o trailing pipelined
            // requests.
            PipelinedRequests::from_complete(
                request
                    .body(Body::from_slice(&request_buffer[offset..]))
                    .unwrap(),
            )
        } else if request_buffer[offset..].len() > content_length {
            // Complete content is captured from the request with trailing pipelined
            // requests.
            PipelinedRequests::from_pipeline(
                request
                    .body(Body::from_slice(&request_buffer[offset..]))
                    .unwrap(),
                Vec::from(&request_buffer[offset + content_length..]),
            )
        } else {
            // Read the rest from TCP stream to form a full request
            let rest = content_length - request_buffer[offset..].len();
            let mut buffer = vec![0u8; rest];
            stream.read_exact(&mut buffer).unwrap();
            request_buffer.extend(&buffer);
            PipelinedRequests::from_complete(
                request
                    .body(Body::from_slice(&request_buffer[offset..]))
                    .unwrap(),
            )
        }
    } else {
        // If the offset points to the end of `requests_buffer` we have a full request,
        // w/o a trailing pipelined request.
        if request_buffer[offset..].is_empty() {
            PipelinedRequests::from_complete(request.body(Body::from_slice(&[])).unwrap())
        } else {
            PipelinedRequests::from_pipeline(
                request.body(Body::from_slice(&[])).unwrap(),
                Vec::from(&request_buffer[offset..]),
            )
        }
    }
}

#[derive(Debug)]
pub(crate) enum ParseRequestError {
    TcpStreamClosed,
    TcpStreamClosedWithoutData,
    HttpParseError(httparse::Error),
    RequestTooLarge,
    UnknownMethod,
}
