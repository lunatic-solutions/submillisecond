# submillisecond

A [lunatic] web framework for the Rust language.

Submillisecond is a **backend** web framework around the Rust language,
[WebAssembly's][wasm] security and the [lunatic scheduler][lunatic_gh].

> This is an early stage project, probably has bugs and the API is still changing. It's also
> important to point out that many Rust crates don't compile to WebAssembly yet and can't be used
> with submillisecond.

If you would like to ask for help or just follow the discussions around Lunatic & submillisecond,
[join our discord server][discord].

# Features

- Fast compilation times
- async-free - All preemption and scheduling is done by [lunatic][lunatic_gh]
- strong security - Each request is handled in a separate _lunatic_ process
- Batteries included
  - Cookies
  - Json
  - Logging
  - Websockets

# Code example

```rust
use submillisecond::{router, Application};

fn index() -> &'static str {
    "Hello :)"
}

fn main() -> std::io::Result<()> {
    Application::new(router! {
        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}
```

## Getting started with lunatic

To run the example you will first need to download the lunatic runtime by following the
installation steps in [this repository][lunatic_gh]. The runtime is just a single executable and runs on
Windows, macOS and Linux. If you have already Rust installed, you can get it with:

```bash
cargo install lunatic-runtime
```

[Lunatic][lunatic_gh] applications need to be compiled to [WebAssembly][wasm] before they can be executed by
the runtime. Rust has great support for WebAssembly and you can build a lunatic compatible
application just by passing the `--target=wasm32-wasi` flag to cargo, e.g:

```bash
# Add the WebAssembly target
rustup target add wasm32-wasi
# Build the app
cargo build --release --target=wasm32-wasi
```

This will generate a .wasm file in the `target/wasm32-wasi/release/` folder inside your project.
You can now run your application by passing the generated .wasm file to Lunatic, e.g:

```
lunatic target/wasm32-wasi/release/<name>.wasm
```

#### Better developer experience

To simplify developing, testing and running lunatic applications with cargo, you can add a
`.cargo/config.toml` file to your project with the following content:

```toml
[build]
target = "wasm32-wasi"

[target.wasm32-wasi]
runner = "lunatic"
```

Now you can just use the commands you are already familiar with, such as `cargo run`, `cargo test`
and cargo is going to automatically build your project as a WebAssembly module and run it inside
`lunatic`.

## Getting started with submillisecond

Add it as a dependency

```toml
submillisecond = "0.2.0-alpha0"
```

## Handlers

Handlers are functions which return a response which implements [`IntoResponse`][intoresponse].

They can have any number of arguments, where each argument is an [extractor].

```rust
fn index(body: Vec<u8>, cookies: Cookies) -> String {
    // ...
}
```

## Routers

Submillisecond provides a [`router!`][router] macro for defining routes in your app.

```rust
#[derive(NamedParam)]
struct User {
    first_name: String,
    last_name: String,
}

fn hi(user: User) -> String {
    format!("Hi {} {}!", user.first_name, user.last_name)
}

fn main() -> std::io::Result<()> {
    Application::new(router! {
        GET "/hi/:first_name/:last_name" => hi
        POST "/update_data" => update_age
    })
    .serve("0.0.0.0:3000")
}
```

The router macro supports:

- [Nested routes](#nested-routes)
- [Url parameters](#url-parameters)
- [Catch-all](#catch-all)
- [Guards](#guards)
- [Middleware](#middleware)

### Nested routes

Routes can be nested.

```rust
router! {
    "/foo" => {
        GET "/bar" => bar
    }
}
```

### Url parameters

Uri parameters can be captured with the [Path] extractor.

```rust
router! {
    GET "/users/:first/:last/:age" => greet
}

fn greet(Path((first, last, age)): Path<(String, String, u32)>) -> String {
    format!("Welcome {first} {last}. You are {age} years old.")
}
```

You can use the [NamedParam] derive macro to define named parameters.

```rust
router! {
    GET "/users/:first/:last/:age" => greet
}

#[derive(NamedParam)]
struct GreetInfo {
    first: String,
    last: String,
    age: u32,
}

fn greet(GreetInfo { first, last, age }: GreetInfo) -> String {
    format!("Welcome {first} {last}. You are {age} years old.")
}
```

Alternatively, you can access the params directly with the [Params] extractor.

### Catch-all

The `_` syntax can be used to catch-all routes.

```rust
router! {
    "/foo" => {
        GET "/bar" => bar
        _ => matches_foo_but_not_bar
    }
    _ => not_found
}
```

### Guards

Routes can be protected by guards.

```rust
struct ContentLengthLimit(u64);

impl Guard for ContentLengthLimit {
    fn check(&self, req: &RequestContext) -> bool {
        // ...
    }
}

router! {
    "/short_requests" if ContentLengthGuard(128) => {
        POST "/super" if ContentLengthGuard(64) => super_short
        POST "/" => short
    }
}
```

Guards can be chained with the `&&` and `||` syntax.

### Middleware

Middleware is any handler which calls [`next_handler`][next_handler] on the request context.
Like handlers, it can use extractors.

```rust
fn logger(req: RequestContext) -> Response {
    println!("Before");
    let result = req.next_handler();
    println!("After");
    result
}

fn main() -> std::io::Result<()> {
    Application::new(router! {
        with logger;

        GET "/" => hi
    })
    .serve("0.0.0.0:3000")
}
```

Middleware can be chained together, and placed within sub-routes.

```rust
router! {
    with [mid1, mid2];

    "/foo" => {
        with [foo_mid1, foo_mid2];
    }
}
```

They can also be specific to a single route.

```rust
router! {
    GET "/" with mid1 => home
}
```

## Testing

Lunatic provides a macro `#[lunatic::test]` to turn your tests into processes. Check out the
[`tests`][tests] folder for examples.

# License

Licensed under either of

- Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (http://opensource.org/licenses/MIT)

at your option.

[lunatic]: https://lunatic.solutions
[lunatic_gh]: https://github.com/lunatic-solutions/lunatic
[wasm]: https://webassembly.org
[discord]: https://discord.gg/b7zDqpXpB4
[tests]: /tests
[router]: https://docs.rs/submillisecond/latest/submillisecond/macro.router.html
[intoresponse]: https://docs.rs/submillisecond/latest/submillisecond/response/trait.IntoResponse.html
[params]: https://docs.rs/submillisecond/latest/submillisecond/params/struct.Params.html
[path]: https://docs.rs/submillisecond/latest/submillisecond/extract/path/struct.Path.html
[namedparam]: https://docs.rs/submillisecond/latest/submillisecond/derive.NamedParam.html
[extractor]: https://docs.rs/submillisecond/latest/submillisecond/extract/trait.FromRequest.html
[next_handler]: https://docs.rs/submillisecond/latest/submillisecond/struct.RequestContext.html#method.next_handler
