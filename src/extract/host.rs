use http::header::{HeaderMap, FORWARDED};

use super::rejection::{FailedToResolveHost, HostRejection};
use super::FromRequest;

const X_FORWARDED_HOST_HEADER_KEY: &str = "X-Forwarded-Host";

/// Extractor that resolves the hostname of the request.
///
/// Hostname is resolved through the following, in order:
/// - `Forwarded` header
/// - `X-Forwarded-Host` header
/// - `Host` header
/// - request target / URI
///
/// Note that user agents can set `X-Forwarded-Host` and `Host` headers to
/// arbitrary values so make sure to validate them to avoid security issues.
#[derive(Debug, Clone)]
pub struct Host(pub String);

impl FromRequest for Host {
    type Rejection = HostRejection;

    fn from_request(req: &mut crate::RequestContext) -> Result<Self, Self::Rejection> {
        let headers = req.headers();

        if let Some(host) = parse_forwarded(headers) {
            return Ok(Host(host.to_owned()));
        }

        if let Some(host) = headers
            .get(X_FORWARDED_HOST_HEADER_KEY)
            .and_then(|host| host.to_str().ok())
        {
            return Ok(Host(host.to_owned()));
        }

        if let Some(host) = headers
            .get(http::header::HOST)
            .and_then(|host| host.to_str().ok())
        {
            return Ok(Host(host.to_owned()));
        }

        if let Some(host) = req.uri().host() {
            return Ok(Host(host.to_owned()));
        }

        Err(HostRejection::FailedToResolveHost(FailedToResolveHost))
    }
}

#[allow(warnings)]
fn parse_forwarded(headers: &HeaderMap) -> Option<&str> {
    // if there are multiple `Forwarded` `HeaderMap::get` will return the first one
    let forwarded_values = headers.get(FORWARDED)?.to_str().ok()?;

    // get the first set of values
    let first_value = forwarded_values.split(',').nth(0)?;

    // find the value of the `host` field
    first_value.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("host")
            .then(|| value.trim().trim_matches('"'))
    })
}

#[cfg(test)]
mod tests {
    use http::header::HeaderName;
    use lunatic::net::TcpStream;

    use super::*;
    use crate::{Body, RequestContext};

    #[lunatic::test]
    fn host_header() {
        let original_host = "some-domain:123";

        let mut req = RequestContext::new(
            http::Request::builder()
                .method("GET")
                .header(http::header::HOST, original_host)
                .body(Body::from_slice(&[]))
                .unwrap(),
            TcpStream::connect("127.0.0.1:22").unwrap(),
        );

        let Host(host) = Host::from_request(&mut req).unwrap();

        assert_eq!(host, original_host);
    }

    #[lunatic::test]
    fn x_forwarded_host_header() {
        let original_host = "some-domain:456";

        let mut req = RequestContext::new(
            http::Request::builder()
                .method("GET")
                .header(X_FORWARDED_HOST_HEADER_KEY, original_host)
                .body(Body::from_slice(&[]))
                .unwrap(),
            TcpStream::connect("127.0.0.1:22").unwrap(),
        );

        let Host(host) = Host::from_request(&mut req).unwrap();

        assert_eq!(host, original_host);
    }

    #[lunatic::test]
    fn x_forwarded_host_precedence_over_host_header() {
        let x_forwarded_host_header = "some-domain:456";
        let host_header = "some-domain:123";

        let mut req = RequestContext::new(
            http::Request::builder()
                .method("GET")
                .header(X_FORWARDED_HOST_HEADER_KEY, x_forwarded_host_header)
                .header(http::header::HOST, host_header)
                .body(Body::from_slice(&[]))
                .unwrap(),
            TcpStream::connect("127.0.0.1:22").unwrap(),
        );

        let Host(host) = Host::from_request(&mut req).unwrap();

        assert_eq!(host, x_forwarded_host_header);
    }

    #[lunatic::test]
    fn uri_host() {
        let mut req = RequestContext::new(
            http::Request::builder()
                .method("GET")
                .uri("127.0.0.1")
                .body(Body::from_slice(&[]))
                .unwrap(),
            TcpStream::connect("127.0.0.1:22").unwrap(),
        );

        let Host(host) = Host::from_request(&mut req).unwrap();

        assert!(host.contains("127.0.0.1"));
    }

    #[lunatic::test]
    fn forwarded_parsing() {
        // the basic case
        let headers = header_map(&[(FORWARDED, "host=192.0.2.60;proto=http;by=203.0.113.43")]);
        let value = parse_forwarded(&headers).unwrap();
        assert_eq!(value, "192.0.2.60");

        // is case insensitive
        let headers = header_map(&[(FORWARDED, "host=192.0.2.60;proto=http;by=203.0.113.43")]);
        let value = parse_forwarded(&headers).unwrap();
        assert_eq!(value, "192.0.2.60");

        // ipv6
        let headers = header_map(&[(FORWARDED, "host=\"[2001:db8:cafe::17]:4711\"")]);
        let value = parse_forwarded(&headers).unwrap();
        assert_eq!(value, "[2001:db8:cafe::17]:4711");

        // multiple values in one header
        let headers = header_map(&[(FORWARDED, "host=192.0.2.60, host=127.0.0.1")]);
        let value = parse_forwarded(&headers).unwrap();
        assert_eq!(value, "192.0.2.60");

        // multiple header values
        let headers = header_map(&[
            (FORWARDED, "host=192.0.2.60"),
            (FORWARDED, "host=127.0.0.1"),
        ]);
        let value = parse_forwarded(&headers).unwrap();
        assert_eq!(value, "192.0.2.60");
    }

    fn header_map(values: &[(HeaderName, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in values {
            headers.append(key, value.parse().unwrap());
        }
        headers
    }
}
