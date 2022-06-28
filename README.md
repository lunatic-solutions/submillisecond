# submillisecond

A [lunatic][3] web framework for Rust.

# Usage example

```rust
use std::io;
use submillisecond::{router, Application};

fn index() -> &'static str {
    "Hello :)"
}

fn main() -> io::Result<()> {
    Application::new(router! {
        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}

```

# Goals

- Amazing developer experience
- Batteries included (Forms, Validation, Auth, ...)
- Database centric (SQLite?)

# Non-goals

- Being the fastest framework out there

# Inspiration

- [IHP][0] - Built in database schema editor that is automatically translated into migrations.
- [Phoenix LiveView][1]
- [Axum][2]

# License

Licensed under either of

- Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (http://opensource.org/licenses/MIT)

at your option.

[0]: https://ihp.digitallyinduced.com
[1]: https://hexdocs.pm/phoenix_live_view/Phoenix.LiveView.html
[2]: https://docs.rs/axum/latest/axum/index.html
[3]: https://lunatic.solutions