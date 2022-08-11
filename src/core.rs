use std::io::Read;

use httparse::{self, Status, EMPTY_HEADER};
use lunatic::net::TcpStream;

const MAX_REQUEST_SIZE: usize = 5 * 1024 * 1024;
const REQUEST_BUFFER_SIZE: usize = 2048;
const MAX_HEADERS: usize = 128;

type RequestResult = Result<http::Request<Vec<u8>>, ParseRequestError>;

/// One or more HTTP request.
///
/// One TCP read can yield multiple pipelined requests. Ideally we would like to
/// process each request in a separate process, but we do an exception for
/// pipelined requests. We can't put data back in the TCP buffer, so we just go
/// ahead and process all requests that one TCP read gives us.
pub(crate) struct PipelinedRequests(Vec<RequestResult>);

impl PipelinedRequests {
    pub(crate) fn request_results(self) -> Vec<RequestResult> {
        self.0
    }
}

pub(crate) fn parse_requests(
    requests_buffer: &mut Vec<u8>,
    mut stream: TcpStream,
) -> PipelinedRequests {
    let mut requests: Vec<RequestResult> = Vec::new();
    let mut buffer = [0_u8; REQUEST_BUFFER_SIZE];

    // Indicates the start of the next pipelined request in the buffer.
    let mut request_start = 0;
    loop {
        // Loop until at least one complete request is read.
        let n = stream.read(&mut buffer).unwrap();
        if n == 0 {
            // In case the TCP stream was closed while processing requests abort
            requests.push(Err(ParseRequestError::TcpStreamClosed));
            return PipelinedRequests(requests);
        }

        // Add read buffer to all requests
        requests_buffer.extend(&buffer[..n]);

        // If request passed max size, abort
        if requests_buffer[request_start..].len() > MAX_REQUEST_SIZE {
            requests.push(Err(ParseRequestError::RequestTooLarge));
            return PipelinedRequests(requests);
        }

        // Try to parse request
        let mut headers = [EMPTY_HEADER; MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(&requests_buffer[request_start..]) {
            Ok(status) => {
                match status {
                    Status::Complete(offset) => {
                        let method = match http::Method::try_from(req.method.unwrap()) {
                            Ok(method) => method,
                            Err(_) => {
                                // If method has an invalid value
                                requests.push(Err(ParseRequestError::UnknownMethod));
                                return PipelinedRequests(requests);
                            }
                        };
                        let request = http::Request::builder()
                            .method(method)
                            .uri(req.path.unwrap());
                        let mut content_lengt = None;
                        let request = req.headers.iter().fold(request, |request, header| {
                            if header.name.to_lowercase() == "content-length" {
                                let value_string = std::str::from_utf8(header.value).unwrap();
                                let length = value_string.parse::<usize>().unwrap();
                                content_lengt = Some(length);
                            }
                            request.header(header.name, header.value)
                        });
                        // If content-length exists, request has body
                        if let Some(content_lengt) = content_lengt {
                            // If the complete content is captured from the
                            // request w/o a trailing pipelined request, finish
                            // request pipelining.
                            if requests_buffer[request_start + offset..].len() == content_lengt {
                                requests.push(Ok(request
                                    .body(Vec::from(&requests_buffer[request_start + offset..]))
                                    .unwrap()));
                                return PipelinedRequests(requests);
                            } else {
                                requests.push(Ok(request
                                    .body(Vec::from(
                                        &requests_buffer[request_start + offset
                                            ..request_start + offset + content_lengt],
                                    ))
                                    .unwrap()));
                                // Force loading of next request in the pipeline
                                request_start += offset + content_lengt;
                            }
                        } else {
                            // If the offset points to the end of requests
                            // buffer we have a full request, w/o a trailing
                            // pipelined request.
                            if requests_buffer[request_start + offset..].is_empty() {
                                requests.push(Ok(request.body(Vec::new()).unwrap()));
                                return PipelinedRequests(requests);
                            } else {
                                requests.push(Ok(request.body(Vec::new()).unwrap()));
                                // Force loading of next request in the pipeline
                                request_start += offset;
                            }
                        }
                    }
                    Status::Partial => {
                        // If the request was incomplete, continue reading from TCP & re-parse.
                        continue;
                    }
                }
            }
            Err(err) => {
                // In case of error return all successfully collected requests until now
                requests.push(Err(ParseRequestError::HttpParseError(err)));
                return PipelinedRequests(requests);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ParseRequestError {
    TcpStreamClosed,
    // BadRequest(http::Error),
    HttpParseError(httparse::Error),
    // InvalidContentLengthHeader,
    // MissingMethod,
    RequestTooLarge,
    UnknownMethod,
}
