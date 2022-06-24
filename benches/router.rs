use criterion::{criterion_group, criterion_main, Criterion};
use http::Method;
use submillisecond::{core::UriReader, params::Params, router, Request};

fn handler() {}

fn router_benchmark_simple(c: &mut Criterion) {
    let router = router! {
        GET "/simple" => handler
    };

    c.bench_function("simple router", |b| {
        b.iter(|| {
            let params = Params::new();
            let reader = UriReader::new("/simple".to_string());
            let request = Request::builder()
                .method(Method::GET)
                .uri("/simple")
                .body(Vec::new())
                .unwrap();

            router(request, params, reader).unwrap();
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
            let params = Params::new();
            let reader = UriReader::new("/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z/one/two/three/four/five/six/seven/eight/nine/ten".to_string());
            let request = Request::builder()
                .method(Method::GET)
                .uri("/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z/one/two/three/four/five/six/seven/eight/nine/ten")
                .body(Vec::new())
                .unwrap();

            router(request, params, reader).unwrap();
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
            let params = Params::new();
            let reader = UriReader::new("/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z/one/two/three/four/five/six/seven/eight/nine/ten".to_string());
            let request = Request::builder()
                .method(Method::GET)
                .uri("/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z/one/two/three/four/five/six/seven/eight/nine/ten")
                .body(Vec::new())
                .unwrap();

            router(request, params, reader).unwrap();
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
