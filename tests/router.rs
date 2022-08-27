use http::Method;
use lunatic::test;
use submillisecond::{http, router, Body, Handler, RequestContext};

macro_rules! build_request {
    ($method: ident, $uri: literal) => {
        build_request!($method, $uri, &[])
    };
    ($method: ident, $uri: literal, $body: expr) => {
        RequestContext::from(
            http::Request::builder()
                .method(Method::$method)
                .uri($uri)
                .body(Body::from_slice($body))
                .unwrap(),
        )
    };
}

macro_rules! handle_request {
    ($router: ident, $method: ident, $uri: literal) => {{
        let req = build_request!($method, $uri);
        Handler::handle(&$router, req)
    }};
    ($router: ident, $method: ident, $uri: literal, $body: expr) => {{
        let req = build_request!($method, $uri, $body);
        Handler::handle(&$router, req)
    }};
}

macro_rules! assert_200 {
    ($res: expr) => {
        assert!(res.status().is_success(), "response wasn't 200");
    };
    ($res: expr, $body: expr) => {
        assert!($res.status().is_success(), "response wasn't 200");
        assert_eq!($body, $res.into_body().as_slice());
    };
}

macro_rules! assert_404 {
    ($res: expr) => {
        assert!(
            $res.status() == http::StatusCode::NOT_FOUND,
            "response wasn't 404, but was:\n{:?}",
            $res
        )
    };
}

fn simple_handler() -> &'static str {
    "OK"
}

#[test]
fn simple_router() {
    let router = router! {
        GET "/" => simple_handler
    };

    // 200
    let res = handle_request!(router, GET, "/");
    assert_200!(res, b"OK");

    // 404
    let res = handle_request!(router, POST, "/");
    assert_404!(res);
}

fn echo_handler(body: Vec<u8>) -> Vec<u8> {
    body
}

#[test]
fn echo_router() {
    let router = router! {
        POST "/echo" => echo_handler
    };

    // 200
    let res = handle_request!(router, POST, "/echo", b"Hello, world!");
    assert_200!(res, b"Hello, world!");

    // 404
    let res = handle_request!(router, GET, "/echo", b"Hello, world!");
    assert_404!(res);
}

#[test]
fn nested_router() {
    let router = router! {
        "/a" => {
            "/b" => {
                GET "/c" => simple_handler
            }
        }
    };

    // 200
    let res = handle_request!(router, GET, "/a/b/c");
    assert_200!(res, b"OK");

    // 404
    let res = handle_request!(router, GET, "/a/b/d");
    assert_404!(res);

    let res = handle_request!(router, GET, "/a/b/c/d");
    assert_404!(res);

    let res = handle_request!(router, GET, "/a/b");
    assert_404!(res);

    let res = handle_request!(router, GET, "/a");
    assert_404!(res);

    let res = handle_request!(router, POST, "/a/b/c");
    assert_404!(res);
}

#[test]
fn param_router() {
    let router = router! {
        "/:a" => {
            "/:b" => {
                "/:c" => {
                    GET "/:one/:two/:three" => simple_handler
                }
            }
        }
    };

    // 200
    let res = handle_request!(router, GET, "/a/b/c/one/two/three");
    assert_200!(res, b"OK");

    // 404
    let res = handle_request!(router, GET, "/a/b/c/one/two");
    assert_404!(res);

    let res = handle_request!(router, GET, "/a/b/c/one/two/three/four");
    assert_404!(res);
}

#[test]
fn fallthrough_router() {
    let router = router! {
        GET "/a" => simple_handler
        GET "/b" => simple_handler
        GET "/c" => simple_handler
        GET "/:foo" => simple_handler
        POST "/:foo" => simple_handler
    };

    // 200
    let res = handle_request!(router, GET, "/a");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/b");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/c");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/hello");
    assert_200!(res, b"OK");

    let res = handle_request!(router, POST, "/hello");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/hello/");
    assert_200!(res, b"OK");

    // 404
    let res = handle_request!(router, GET, "/a/b");
    assert_404!(res);
}

#[test]
fn all_methods_router() {
    let router = router! {
        GET "/get" => simple_handler
        POST "/post" => simple_handler
        PUT "/put" => simple_handler
        DELETE "/delete" => simple_handler
        HEAD "/head" => simple_handler
        OPTIONS "/options" => simple_handler
        PATCH "/patch" => simple_handler
    };

    // 200
    let res = handle_request!(router, GET, "/get");
    assert_200!(res, b"OK");

    let res = handle_request!(router, POST, "/post");
    assert_200!(res, b"OK");

    let res = handle_request!(router, PUT, "/put");
    assert_200!(res, b"OK");

    let res = handle_request!(router, DELETE, "/delete");
    assert_200!(res, b"OK");

    let res = handle_request!(router, HEAD, "/head");
    assert_200!(res, b"OK");

    let res = handle_request!(router, OPTIONS, "/options");
    assert_200!(res, b"OK");

    let res = handle_request!(router, PATCH, "/patch");
    assert_200!(res, b"OK");

    // 404
    let res = handle_request!(router, GET, "/post");
    assert_404!(res);

    let res = handle_request!(router, POST, "/put");
    assert_404!(res);

    let res = handle_request!(router, PUT, "/delete");
    assert_404!(res);

    let res = handle_request!(router, DELETE, "/head");
    assert_404!(res);

    let res = handle_request!(router, HEAD, "/options");
    assert_404!(res);

    let res = handle_request!(router, OPTIONS, "/patch");
    assert_404!(res);

    let res = handle_request!(router, PATCH, "/get");
    assert_404!(res);
}

#[test]
fn wildcard_router() {
    let router = router! {
        GET "/" => simple_handler
        GET "/foo" => simple_handler
        GET "/any-*" => simple_handler
    };

    // 200
    let res = handle_request!(router, GET, "/");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/foo");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/any-");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/any-thing");
    assert_200!(res, b"OK");

    let res = handle_request!(router, GET, "/any-thing/at/all");
    assert_200!(res, b"OK");

    // 400
    let res = handle_request!(router, GET, "/abc");
    assert_404!(res);

    let res = handle_request!(router, GET, "/abc/def");
    assert_404!(res);

    let res = handle_request!(router, GET, "/fooo");
    assert_404!(res);

    let res = handle_request!(router, GET, "/any");
    assert_404!(res);
}
