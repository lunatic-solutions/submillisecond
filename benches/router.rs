use criterion::{criterion_group, criterion_main, Criterion};
use http::Method;
use submillisecond::{router, Body, Handler};

fn handler() {}

fn router_benchmark_simple(c: &mut Criterion) {
    let router = router! {
        GET "/simple" => handler
    };

    c.bench_function("simple router", |b| {
        b.iter(|| {
            let req = http::Request::builder()
                .method(Method::GET)
                .uri("/simple")
                .body(Body::from_slice(&[]))
                .unwrap();

            let res = Handler::handle(&router, req.into());
            assert!(res.status().is_success());
        })
    });
}

fn router_benchmark_nested(c: &mut Criterion) {
    let router = router! {
        "/a" => {
            "/b" => {
                "/c" => {
                    "/d" => {
                        "/e" => {
                            "/f" => {
                                "/g" => {
                                    "/h" => {
                                        "/i" => {
                                            "/j" => {
                                                "/k" => {
                                                    "/l" => {
                                                        "/m" => {
                                                            "/n" => {
                                                                "/o" => {
                                                                    "/p" => {
                                                                        "/q" => {
                                                                            "/r" => {
                                                                                "/s" => {
                                                                                    "/t" => {
                                                                                        "/u" => {
                                                                                            "/v" => {
                                                                                                "/w" => {
                                                                                                    "/x" => {
                                                                                                        "/y" => {
                                                                                                            "/z" => {
                                                                                                                GET "/one/two/three/four/five/six/seven/eight/nine/ten" => handler
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    c.bench_function("nested router", |b| {
        b.iter(|| {
            let req = http::Request::builder()
                .method(Method::GET)
                .uri("/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z/one/two/three/four/five/six/seven/eight/nine/ten")
                .body(Body::from_slice(&[]))
                .unwrap();

            let res = Handler::handle(&router, req.into());
            assert!(res.status().is_success());
        })
    });
}

fn router_benchmark_params(c: &mut Criterion) {
    let router = router! {
        "/:a" => {
            "/:b" => {
                "/:c" => {
                    "/:d" => {
                        "/:e" => {
                            "/:f" => {
                                "/:g" => {
                                    "/:h" => {
                                        "/:i" => {
                                            "/:j" => {
                                                "/:k" => {
                                                    "/:l" => {
                                                        "/:m" => {
                                                            "/:n" => {
                                                                "/:o" => {
                                                                    "/:p" => {
                                                                        "/:q" => {
                                                                            "/:r" => {
                                                                                "/:s" => {
                                                                                    "/:t" => {
                                                                                        "/:u" => {
                                                                                            "/:v" => {
                                                                                                "/:w" => {
                                                                                                    "/:x" => {
                                                                                                        "/:y" => {
                                                                                                            "/:z" => {
                                                                                                                GET "/:one/:two/:three/:four/:five/:six/:seven/:eight/:nine/:ten" => handler
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    c.bench_function("params router", |b| {
        b.iter(|| {
            let req = http::Request::builder()
                .method(Method::GET)
                .uri("/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z/one/two/three/four/five/six/seven/eight/nine/ten")
                .body(Body::from_slice(&[]))
                .unwrap();

            let res = Handler::handle(&router, req.into());
            assert!(res.status().is_success());
        })
    });
}

criterion_group!(
    benches,
    router_benchmark_simple,
    router_benchmark_nested,
    router_benchmark_params
);
criterion_main!(benches);
